//! clippy-bench — the frozen real-code healing bench + score history (Plan 014).
//!
//! ## Why this exists
//!
//! The repo had no number that can falsify "we're getting better":
//! retrieval precision is measured on authored fixtures coupled to the
//! corpus (kernel rows are self-variant retrieval — the corpus retrieving
//! renamed copies of itself), rule count is a coverage metric that grows by
//! construction, and Bench 009's 0/10 held-out measures the L1 verbatim
//! cache tier, not the proposer path production runs. This bench freezes a
//! REAL-CODE fixture set (mined from pre-sweep sibling commits, provenance
//! recorded), heals it with the SAME bounded-fixpoint loop as `cargo refine
//! --fix`, and scores the result against real `cargo clippy` verdicts — an
//! oracle with zero relationship to our corpus or proposers (T3.5.5).
//!
//! The denominator is FROZEN: corpus growth cannot move the score. The
//! score history (`.benchmarks/score_history.jsonl`) + SVG curve
//! (`.benchmarks/score_history.svg`) make improvement — or regression —
//! visible per iteration.
//!
//! ## Scoring (per lint-NAME presence; the oracle dedupes)
//!
//! | axis | definition |
//! |---|---|
//! | `heal_rate` | Σ resolved vocabulary lints / Σ vocabulary lints before |
//! | `created_total` | Σ clippy lints (ANY, not just vocabulary) newly present after healing — the false-positive axis |
//! | `parse_safety` | fixtures whose healed text still parses / fixtures edited |
//! | `decline_correctness` | clean fixtures (0 vocab lints) left untouched by the clippy lane / clean fixtures |
//!
//! Declining is a correct answer: the L4 relevance lesson (Issue 030)
//! applied to the modelless lanes — confabulated edits on clean code are
//! worse than no edits.
//!
//! ## Honest scope (v1)
//!
//! - Fixtures must compile in a BARE stub crate (the oracle's `compiles`
//!   flag is the filter), so crate-internal spans are excluded by
//!   construction. Per-fixture stub manifests are Plan 014 Phase 3.
//! - Provenance is sweep-history: code classes the development process has
//!   seen. v1 anchors the curve + guards regressions; truly-unseen sources
//!   (crates.io) are Phase 3.
//! - Ground truth is RE-DERIVED every run (clippy's lint set moves between
//!   toolchains — the oracle's caveat). Disagreement with the frozen
//!   `expect_lints` prints a loud drift warning; the fresh verdict wins.

pub mod corruption;
pub mod corruption_corpus_v9;
pub mod cross_ooo;
pub mod fixtures;
pub mod generations;
pub mod localization;
pub mod necessity;
pub mod ood;
pub mod report;

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;

use crate::domains::clippy_lints::proposers::FIXABLE_LINTS;
use crate::domains::rust_perf::proposers::FIXABLE_RULES;
use crate::draft::clippy_oracle::ClippyOracle;
use crate::draft::clippy_oracle::OracleVerdict;
use crate::edit_emitter::apply_edits_traced;

/// One frozen bench fixture: a real code span + its provenance.
#[derive(Debug, Clone)]
pub struct BenchFixture {
    /// Stable unique id (`repo-sha-path-N`).
    pub id: &'static str,
    /// `repo@sha:path` — where the span was mined from.
    pub provenance: &'static str,
    /// The span source, verbatim (compiles standalone in a bare lib crate).
    pub code: &'static str,
    /// Vocabulary lints real clippy fired at FREEZE time (drift check only —
    /// the run re-derives and the fresh verdict wins).
    pub expect_lints: &'static [&'static str],
    /// `true` = decline fixture (expected 0 vocabulary lints + 0 clippy-lane
    /// edits). `false` = healable fixture.
    pub clean: bool,
}

/// The clippy-lane vocabulary: lints our proposers claim to fix.
#[must_use]
pub fn vocabulary() -> BTreeSet<String> {
    FIXABLE_LINTS.iter().map(ToString::to_string).collect()
}

/// Construct the bench's oracle (distinct temp root so a concurrent
/// `self_evolve` run never shares a crate dir with a bench run).
/// The root itself is DELIBERATE — `[documented-fixed-root]`: keyed crate
/// dirs under it; see `clippy_lints::verify::ClippyVerifier` (katgpt-rs
/// Issue 832 T5).
#[must_use]
pub fn draft_oracle() -> ClippyOracle {
    ClippyOracle::new().with_work_root(std::env::temp_dir().join("riir_clippy_score_bench"))
}

/// Wrap fixture code as the oracle stub crate's `lib.rs`.
///
/// Inner `allow` pragmas silence rustc's dead-code/unused noise (this is a
/// span, not a program) WITHOUT touching any clippy lint — the same wrap is
/// used at mine time and bench time so labels can't drift between them.
#[must_use]
pub fn wrap(code: &str) -> String {
    format!("#![allow(dead_code)]\n#![allow(unused)]\n{code}")
}

// ── Issue 089 T6: the provenance-stratified score-bench read ───────────

/// The provenance CLASS of a bench fixture — the coarse axis the stratified
/// read aggregates over (the family, via [`super::ood::provenance_family`],
/// is the fine-grained `repo@sha` identity; the class is the organic-vs-
/// synthetic question the frontier curriculum cares about, the same axis
/// Issue 089 T5 put on the pair + outcome feeds).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProvenanceClass {
    /// `repo@sha` family — a span mined from a real repo state (organic
    /// feature churn: `katgpt-rs@9e0e6a57`, `riir-train@a1197e06~1`, …).
    Repo,
    /// `authored-composite` family — hand-built multi-lint composites.
    Authored,
    /// `construction` family — generated/planted ladders (`construction:gen…`,
    /// the v2/v3/v7 frozen-conversion tag; the same word the pair vocabulary
    /// uses, deliberately).
    Construction,
    /// Anything else — counted loud, never silently dropped (the frontier
    /// fail-closed convention).
    Other,
}

impl ProvenanceClass {
    /// The stable render + wire label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Repo => "repo",
            Self::Authored => "authored",
            Self::Construction => "construction",
            Self::Other => "other",
        }
    }
}

/// Classify one provenance FAMILY (the prefix [`super::ood::provenance_family`]
/// extracts — classify the FAMILY, not the full string, or the `:`-suffix
/// defeats the exact matches).
#[must_use]
pub fn provenance_class(family: &str) -> ProvenanceClass {
    match family {
        "authored-composite" => ProvenanceClass::Authored,
        "construction" => ProvenanceClass::Construction,
        f if f.contains('@') => ProvenanceClass::Repo,
        _ => ProvenanceClass::Other,
    }
}

/// One stratum of the stratified read: this run's outcomes aggregated over
/// one provenance class. Heals the v1 blind spot — the frozen set mixes
/// repo-mined spans and authored composites into ONE `heal_rate`; the class
/// split shows whether the rate holds on real-mined material (the bench-side
/// form of T5's outcome-feed question).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceStratum {
    /// The class label ([`ProvenanceClass::label`]).
    pub class: String,
    /// Fixtures of this class that ran (excluded ones count nowhere).
    pub n_ran: usize,
    /// Ran fixtures with ≥1 vocabulary finding before healing.
    pub n_healable: usize,
    /// Ran fixtures with zero vocabulary findings (the decline axis).
    pub n_clean: usize,
    /// Σ vocabulary findings before.
    pub vocab_before: usize,
    /// Σ findings resolved by healing.
    pub resolved: usize,
    /// Σ lints (any) newly created by healing.
    pub created: usize,
    /// `resolved / vocab_before` (0.0 when the class had no findings) — the
    /// aggregate `heal_rate` semantics, per class.
    pub heal_rate: f32,
}

/// Stratify one finished bench run by fixture provenance class (Issue 089
/// T6 — the honest half of the task; the D-formula half is declined on the
/// record in the issue). Pure arithmetic over the run's outcomes; the
/// fixture join resolves families through [`super::ood::ood_set_family`]
/// (the mining join — v6 ids redirect to their real `repo@sha` provenance,
/// the frozen-construction tag is freeze-gate metadata, not mining truth).
///
/// Deterministic: canonical class order (repo, authored, construction,
/// other), classes with zero RAN fixtures omitted (an empty class is not a
/// measurement — the ood families precedent lists only present families).
/// An outcome whose id matches no fixture is defensive-counted into `other`
/// (never silently dropped — the run and the stratify call share one
/// fixture list, so a miss is a bug this makes visible instead of hiding).
#[must_use]
// Fractions over usize/u32 counters — the report's quantity; allowed.
#[allow(clippy::cast_precision_loss)]
pub fn stratify(fixture_list: &[BenchFixture], score: &BenchScore) -> Vec<ProvenanceStratum> {
    // id → mining family (the join the ood axis already owns).
    let families: BTreeMap<&str, &str> = fixture_list
        .iter()
        .map(|fx| (fx.id, ood::ood_set_family(fx)))
        .collect();
    // Per-class accumulators: (ran, healable, clean, vocab_before, resolved, created).
    let mut acc: BTreeMap<ProvenanceClass, (usize, usize, usize, usize, usize, usize)> =
        BTreeMap::new();
    for o in &score.outcomes {
        if !o.ran {
            continue;
        }
        let family = families.get(o.id.as_str()).copied().unwrap_or("other");
        let class = provenance_class(family);
        let e = acc.entry(class).or_insert((0, 0, 0, 0, 0, 0));
        e.0 += 1;
        let vb = o.vocab_before.len();
        if vb == 0 {
            e.2 += 1;
        } else {
            e.1 += 1;
        }
        e.3 += vb;
        e.4 += o.resolved();
        e.5 += o.created.len();
    }
    let canonical = [
        ProvenanceClass::Repo,
        ProvenanceClass::Authored,
        ProvenanceClass::Construction,
        ProvenanceClass::Other,
    ];
    canonical
        .into_iter()
        .filter_map(|class| {
            let (n_ran, n_healable, n_clean, vocab_before, resolved, created) =
                acc.remove(&class)?;
            if n_ran == 0 {
                return None;
            }
            Some(ProvenanceStratum {
                class: class.label().to_string(),
                n_ran,
                n_healable,
                n_clean,
                vocab_before,
                resolved,
                created,
                heal_rate: if vocab_before == 0 {
                    0.0
                } else {
                    resolved as f32 / vocab_before as f32
                },
            })
        })
        .collect()
}

/// Per-fixture run outcome (diagnostics for the console report).
#[derive(Debug, Clone, Default)]
pub struct FixtureOutcome {
    pub id: String,
    /// Oracle ran for both before/after (false → excluded, cargo missing).
    pub ran: bool,
    /// Freshly-derived vocabulary lints before healing.
    pub vocab_before: BTreeSet<String>,
    /// ALL clippy lints before (vocabulary + outside).
    pub all_before: BTreeSet<String>,
    /// ALL clippy lints after healing.
    pub all_after: BTreeSet<String>,
    /// Vocabulary lints still present after healing.
    pub vocab_persist: BTreeSet<String>,
    /// Clippy lints newly created by healing (any, not just vocabulary).
    pub created: BTreeSet<String>,
    /// Total proposer edits applied across the fixpoint.
    pub edits: usize,
    /// Fixpoint passes that applied edits.
    pub passes: usize,
    /// Healed text parses (`syn::parse_file`).
    pub parses: bool,
    /// Frozen-label drift detected (fresh != frozen).
    pub drifted: bool,
}

impl FixtureOutcome {
    /// Vocabulary lints resolved by healing.
    #[must_use]
    pub fn resolved(&self) -> usize {
        self.vocab_before.difference(&self.vocab_persist).count()
    }

    /// Did the clippy lane touch a clean fixture? (decline violation)
    #[must_use]
    pub fn clean_violation(&self) -> bool {
        self.vocab_before.is_empty() && self.edits > 0
    }

    /// The shaped verdict (Issue 089 T7): `1.0` clean clear (targets
    /// resolved, nothing created), [`SHAPED_VERDICT`] "target cleared ∧ new
    /// diags introduced" (a half win — full credit would reward feeding
    /// the created axis, zero would ignore the target it did clear), `0.0`
    /// otherwise (persist / broken output / declined). Fixture-attributed:
    /// every resolved finding of the fixture earns the fixture's verdict —
    /// per-edit attribution would need edit→diagnostic tracing, so the v8
    /// generation keeps this exact BY CONSTRUCTION (one target lint per
    /// atom; multi-target fixtures score the same way, documented here).
    #[must_use]
    pub fn shaped_verdict(&self) -> f32 {
        if !self.parses || self.resolved() == 0 {
            return 0.0;
        }
        if self.created.is_empty() {
            1.0
        } else {
            SHAPED_VERDICT
        }
    }

    /// Is this outcome the shaped case (`verdict == SHAPED_VERDICT`)?
    #[must_use]
    pub fn is_shaped(&self) -> bool {
        self.parses && self.resolved() > 0 && !self.created.is_empty()
    }
}

/// Aggregate bench score — feeds one `score_history.jsonl` record.
#[derive(Debug, Clone, Default)]
pub struct BenchScore {
    pub n_total: usize,
    pub n_ran: usize,
    pub n_excluded: usize,
    pub n_clean: usize,
    /// Σ resolved / Σ vocab-before over healable fixtures that ran.
    pub heal_rate: f32,
    /// Σ created lints (any) across all ran fixtures.
    pub created_total: usize,
    /// Created per vocab-before finding (lower is better).
    pub created_rate: f32,
    /// Parses / edited fixtures.
    pub parse_safety: f32,
    /// Clean fixtures untouched / clean fixtures that ran.
    pub decline_correctness: f32,
    pub edits_total: usize,
    pub vocab_size: usize,
    /// Persisting (lint, count) — the miss diagnostic, most-frequent first.
    pub persist: Vec<(String, usize)>,
    pub outcomes: Vec<FixtureOutcome>,
    // ── Issue 089 T7: the shaped-verdict axis (instrumentation, not a
    // scored axis — the admission/ratchet math keeps reading heal_rate;
    // the shaped axis prices the half-win the plain rate over-credits) ──
    /// Ran healable fixtures whose verdict is exactly [`SHAPED_VERDICT`].
    pub n_shaped: usize,
    /// Σ verdict × resolved findings — the shaped-weighted credit
    /// ([`FixtureOutcome::shaped_verdict`] per fixture).
    pub shaped_credit: f32,
    /// `shaped_credit / Σ vocab-before` — strictly ≤ `heal_rate`.
    pub shaped_rate: f32,
    /// Per-RULE shaped counts (Issue 089 T7's "Beta (half-count)" feed):
    /// the target rule of each shaped fixture earns one row entry worth
    /// `+0.5 r AND +0.5 f`. Attribution is unambiguous only for
    /// single-target fixtures — a shaped fixture with ≠1 `vocab_before`
    /// lint counts in the rates above but NOT here (recorded, not guessed).
    pub shaped_half_counts: Vec<(String, u32)>,
}

/// The shaped-verdict value (Issue 089 T7): a heal that clears its target
/// AND introduces new diagnostics is a HALF WIN — see
/// [`FixtureOutcome::shaped_verdict`].
pub const SHAPED_VERDICT: f32 = 0.5;

// ── Issue 089 T8: the per-rule outcome read (per-candidate evidence) ────

/// One rule's outcome over this run's single-target fixtures (Issue 089
/// T8 — the emission half of the α-fit blocker: `score_history.jsonl`
/// carried AGGREGATE heal rates per run, so nothing could be replayed per
/// candidate; these rows are what a grid-fit replays, recorded AT BENCH
/// TIME). Attribution follows the T7 `shaped_half_counts` law:
/// single-target fixtures only (a ran fixture whose `vocab_before` is
/// exactly one lint); multi-target composites and clean fixtures count in
/// the aggregate axes, never here — recorded via
/// [`RuleOutcomeRead::skipped_multi_target`], never guessed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleOutcome {
    /// The target lint/rule name.
    pub rule: String,
    /// Ran single-target fixtures of this rule. Name-presence scoring: the
    /// bench scores a fixture per lint NAME, so this IS the rule's finding
    /// count this run.
    pub fixtures: usize,
    /// Fixtures whose target resolved (name absent from `vocab_persist`).
    pub resolved: usize,
    /// Fixtures still firing after healing (output ok — the persist fork of
    /// T4's triage).
    pub persist: usize,
    /// Fixtures whose healed output broke — `FixtureOutcome::parses` is the
    /// FULL output-ok signal (syn-parse ∧ oracle-ran ∧ compiles), so this
    /// is the complete broken fork; `vocab_persist` mirrors `vocab_before`
    /// there by construction.
    pub broken: usize,
    /// Σ created lints (any) across this rule's fixtures.
    pub created: usize,
    /// Σ proposer edits across this rule's fixtures — the log-length proxy
    /// the R(n) tie-break consumes (`R = 1 if n≤N else 1−α·ln(n/N)`).
    pub edits: usize,
    /// Fixtures with the [`SHAPED_VERDICT`] half-win verdict.
    pub shaped: usize,
}

/// The per-rule read of one finished bench run (Issue 089 T8).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RuleOutcomeRead {
    /// Per-rule rows, rule ascending (`BTreeMap` iteration order — the
    /// `shaped_half_counts` determinism law).
    pub rows: Vec<RuleOutcome>,
    /// Ran healable fixtures with ≥2 vocabulary lints — unattributable per
    /// rule (counted in the aggregate axes, loud here, never guessed).
    pub skipped_multi_target: usize,
}

/// The per-rule loop accumulator (named fields — the `RunAccum` precedent;
/// a 7-tuple would trip `type_complexity` and read by index).
#[derive(Default)]
struct RuleAccum {
    fixtures: usize,
    resolved: usize,
    persist: usize,
    broken: usize,
    created: usize,
    edits: usize,
    shaped: usize,
}

impl RuleAccum {
    fn record(&mut self, o: &FixtureOutcome) {
        self.fixtures += 1;
        if !o.parses {
            self.broken += 1; // output-ok is the full signal
        } else if o.resolved() > 0 {
            self.resolved += 1; // single target ⇒ resolved() ∈ {0, 1}
        } else {
            self.persist += 1; // still firing
        }
        self.created += o.created.len();
        self.edits += o.edits;
        if o.is_shaped() {
            self.shaped += 1;
        }
    }
}

/// Decompose one finished bench run into per-rule outcomes (Issue 089 T8).
/// Pure arithmetic over [`BenchScore::outcomes`] — the `stratify`
/// precedent (post-hoc over the run, no hot-loop change). Per-rule fork
/// invariant: `fixtures == resolved + persist + broken` for every row.
#[must_use]
pub fn rule_outcomes(score: &BenchScore) -> RuleOutcomeRead {
    let mut acc: BTreeMap<String, RuleAccum> = BTreeMap::new();
    let mut skipped_multi_target = 0usize;
    for o in &score.outcomes {
        if !o.ran || o.vocab_before.is_empty() {
            continue; // excluded (never ran) / clean (the decline axis, not per-rule)
        }
        if o.vocab_before.len() > 1 {
            skipped_multi_target += 1;
            continue;
        }
        let Some(rule) = o.vocab_before.iter().next().cloned() else {
            continue;
        };
        acc.entry(rule).or_default().record(o);
    }
    RuleOutcomeRead {
        rows: acc
            .into_iter()
            .map(|(rule, a)| RuleOutcome {
                rule,
                fixtures: a.fixtures,
                resolved: a.resolved,
                persist: a.persist,
                broken: a.broken,
                created: a.created,
                edits: a.edits,
                shaped: a.shaped,
            })
            .collect(),
        skipped_multi_target,
    }
}

/// The production `--fix` heal loop, mirrored from `src/bin/heal.rs`
/// (`MAX_FIX_PASSES = 4`, perf + clippy lanes, overlap-selected splices).
///
/// Two deliberate divergences, both documented:
/// - `fxhashmap` is SKIPPED: the oracle stub crate has no rustc-hash dep —
///   mirroring the production dep gate for a crate without it.
/// - No fmt pass: the bench measures the proposers, not rustfmt.
#[must_use]
pub fn heal_fixpoint(source: &str) -> (String, usize, usize) {
    heal_fixpoint_with(source, &[])
}

/// [`heal_fixpoint`] with an optional CANDIDATE textual lane (Proposal 005
/// T2.4 — the promotion gate's trial mechanism): each pass first applies
/// every candidate's verbatim substitution (`buggy` → `fixed`, ALL
/// occurrences — the L1 verbatim-tier semantics a corpus row carries), then
/// the proposer edits over the result. Candidates + proposers compose in
/// ONE bounded fixpoint; a candidate whose `fixed` re-introduces a buggy
/// shape simply gets re-healed by the proposers on the next pass.
///
/// A candidate substitution that breaks parsing ends the loop with the
/// broken text — exactly the applied-breakage the promotion gate's
/// heal-rate bar exists to catch (a non-compiling after-span scores as
/// NOTHING resolved, never as healed).
///
/// Empty `candidates` is byte-identical to [`heal_fixpoint`].
#[must_use]
pub fn heal_fixpoint_with(source: &str, candidates: &[(&str, &str)]) -> (String, usize, usize) {
    heal_fixpoint_cfg(source, candidates, &ComponentOverride::default())
}

// ── Issue 108 T2: the component-necessity intervention seam ──────────────

/// The component-intervention spec at the proposer-dispatch seam (Issue 108
/// — the CVRR protocol, `arXiv:2609.06746`: "informativeness ≠ use").
/// Semantics consume the katgpt-rs Issue-776 intervention substrate's
/// PROTOCOL — see the T1 audit in `.issues/108` for the honest mapping
/// (the buffer-level operators apply to latent memory; the healer's fix
/// lane is dispatch-level, so what composes here is the arm semantics +
/// the verdict-predicate shapes):
///
/// - **empty arm** (`perturb_empty`): [`ComponentOverride::disabled`] — the
///   rule's slot contributes zero edits, exactly an unknown rule
///   (`_ => Vec::new()` in both domains' dispatch).
/// - **matched-swap arm** (`perturb_matched_swap`): [`ComponentOverride::swap`]
///   — the rule's slot redirects to a donor's `collect_edits` on the SAME
///   source state: a coherent whole-contribution swap. "Matched" =
///   same-domain donor (dispatched through the same domain's collect fn as
///   the slot it replaces).
///
/// The verdict shapes mirror
/// `katgpt_core::interpolation_geometry::InterventionReport`: `latent_is_causal`
/// → a nonzero Δ(resolved-rate) under intervention (load-bearing);
/// `flips_to_donor` → the outcome vector moves toward the donor arm's.
/// Both are computed by the necessity harness ([`necessity`]), not here.
///
/// Empty override = the untouched composition — byte-identical heal,
/// pinned by test (the non-vacuity law: the instrument must be a no-op
/// when it claims nothing).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComponentOverride {
    /// Rules whose dispatch slot is EMPTIED: zero edits, no error — the
    /// unknown-rule path by construction.
    pub disabled: BTreeSet<String>,
    /// rule → donor: the rule's dispatch slot redirects to the donor's
    /// collect fn. The donor's OWN slot is untouched (independent — a swap
    /// does not also disable the donor).
    pub swap: BTreeMap<String, String>,
}

impl ComponentOverride {
    /// The dispatch id for a rule's slot: `None` = disabled (skip the
    /// rule), `Some(id)` = the id to dispatch (the rule itself, or its
    /// swap donor). Unknown names are inert (they simply never match a
    /// slot), so a planted no-op control cannot panic here.
    #[must_use]
    pub fn dispatch<'a>(&'a self, rule: &'a str) -> Option<&'a str> {
        if self.disabled.contains(rule) {
            return None;
        }
        match self.swap.get(rule) {
            Some(donor) => Some(donor.as_str()),
            None => Some(rule),
        }
    }
}

/// [`heal_fixpoint_with`] + the component-intervention lane (Issue 108 T2):
/// each rule's dispatch consults [`ComponentOverride`] — disabled rules
/// skip (zero edits), swapped rules dispatch their donor. The candidate
/// textual lane composes first exactly as in [`heal_fixpoint_with`].
/// Empty override is byte-identical to [`heal_fixpoint`] (pinned by test).
#[must_use]
pub fn heal_fixpoint_cfg(
    source: &str,
    candidates: &[(&str, &str)],
    ovr: &ComponentOverride,
) -> (String, usize, usize) {
    const MAX_FIX_PASSES: usize = 4;
    let mut current = source.to_string();
    let mut edits_applied = 0usize;
    let mut passes = 0usize;
    for _ in 0..MAX_FIX_PASSES {
        // Candidate lane first: the incoming rule gets its chance on its
        // exact shapes before the proposers compose over the result.
        let mut candidate_edits = 0usize;
        for (buggy, fixed) in candidates {
            let hits = current.match_indices(buggy).count();
            if hits > 0 {
                current = current.replace(buggy, fixed);
                candidate_edits += hits;
            }
        }
        if candidate_edits > 0 {
            edits_applied += candidate_edits;
            passes += 1;
        }
        let Ok(parsed) = syn::parse_file(&current) else {
            break;
        };
        let mut edits = Vec::new();
        for rule in FIXABLE_RULES {
            if *rule == "fxhashmap" {
                continue;
            }
            // Issue 108: the intervention seam — `None` skips the rule
            // (the empty arm), `Some(donor)` swaps the contribution.
            let Some(dispatch_id) = ovr.dispatch(rule) else {
                continue;
            };
            edits.extend(crate::domains::rust_perf::proposers::collect_edits(
                dispatch_id, &current, &parsed,
            ));
        }
        for lint in FIXABLE_LINTS {
            let Some(dispatch_id) = ovr.dispatch(lint) else {
                continue;
            };
            edits.extend(crate::domains::clippy_lints::proposers::collect_edits(
                dispatch_id, &current, &parsed,
            ));
        }
        if edits.is_empty() {
            break;
        }
        let (next, splices) =
            apply_edits_traced(&current, edits.iter().map(|re| re.edit.clone()).collect());
        let changed = next != current;
        edits_applied += splices.len();
        if !splices.is_empty() {
            passes += 1;
        }
        current = next;
        if !changed {
            break;
        }
    }
    (current, edits_applied, passes)
}

/// The after-heal verdict half of one fixture: never score broken output
/// as healed.
///
/// Returns `(after_lints, output_ok, vocab_persist, created)` where a
/// non-compiling healed span counts as NOTHING resolved (`vocab_persist =
/// vocab_before`) — scoring an empty after-set as "everything resolved"
/// would reward breakage.
fn after_verdict(
    oracle: &ClippyOracle,
    healed: &str,
    vocab: &BTreeSet<String>,
    vocab_before: &BTreeSet<String>,
    before_lints: &BTreeSet<String>,
) -> (BTreeSet<String>, bool, BTreeSet<String>, BTreeSet<String>) {
    let after = oracle.lints_for(&wrap(healed));
    let after_ok = after.ran && after.compiles;
    let after_lints = if after_ok {
        after.lints
    } else {
        BTreeSet::new()
    };
    let output_ok = syn::parse_file(healed).is_ok() && after_ok;
    let vocab_persist: BTreeSet<String> = if after_ok {
        after_lints.intersection(vocab).cloned().collect()
    } else {
        vocab_before.clone()
    };
    let created: BTreeSet<String> = if after_ok {
        after_lints.difference(before_lints).cloned().collect()
    } else {
        BTreeSet::new()
    };
    (after_lints, output_ok, vocab_persist, created)
}

/// Run the full bench. Fixtures whose oracle verdict did not run (cargo
/// missing) or did not compile (toolchain drift broke the stub) are
/// EXCLUDED loudly, never silently scored.
// Scoring rates are count→ratio casts over fixture counts (< 10⁴) — far
// below f32's mantissa; allowed with cause.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn run_bench(oracle: &ClippyOracle, fixture_list: &[BenchFixture]) -> BenchScore {
    run_bench_with(oracle, fixture_list, &[])
}

/// [`run_bench`] with a candidate textual lane (Proposal 005 T2.4): the
/// FROZEN fixtures stay frozen — only the heal loop gains the candidate
/// substitutions (see [`heal_fixpoint_with`]). The `before` verdicts are
/// derived from the ORIGINAL fixture code in both arms, so baseline and
/// trial share identical denominators by construction — the promotion
/// gate compares integer totals, never floats.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn run_bench_with(
    oracle: &ClippyOracle,
    fixture_list: &[BenchFixture],
    candidates: &[(&str, &str)],
) -> BenchScore {
    run_bench_inner(
        oracle,
        fixture_list,
        candidates,
        &ComponentOverride::default(),
        None,
    )
}

/// [`run_bench`] under a component intervention (Issue 108 T2 — the
/// necessity harness's arm runner): frozen fixtures, frozen BEFORE
/// verdicts, only the composition changes. The before cache is keyed by
/// fixture id and MUST be derived from the same fixture list (a miss
/// falls back to a live derive — correct, just unpaid-for).
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn run_bench_overridden(
    oracle: &ClippyOracle,
    fixture_list: &[BenchFixture],
    ovr: &ComponentOverride,
    before_cache: &BTreeMap<String, OracleVerdict>,
) -> BenchScore {
    run_bench_inner(oracle, fixture_list, &[], ovr, Some(before_cache))
}

/// The shared bench body: the candidate lane (Proposal 005 T2.4) and the
/// component-intervention lane (Issue 108 T2) compose; the optional
/// before-cache (Issue 108 T2) lets N intervention arms reuse ONE before
/// pass — the before verdict is a fixture property (frozen code, no
/// intervention touches it), so re-deriving it per arm is a pure 2× cost.
#[allow(clippy::cast_precision_loss)]
fn run_bench_inner(
    oracle: &ClippyOracle,
    fixture_list: &[BenchFixture],
    candidates: &[(&str, &str)],
    ovr: &ComponentOverride,
    before_cache: Option<&BTreeMap<String, OracleVerdict>>,
) -> BenchScore {
    let vocab = vocabulary();
    let mut score = BenchScore {
        n_total: fixture_list.len(),
        vocab_size: vocab.len(),
        ..BenchScore::default()
    };
    let mut acc = RunAccum::default();

    for fx in fixture_list {
        let before: OracleVerdict = match before_cache.and_then(|c| c.get(fx.id)) {
            Some(cached) => cached.clone(),
            None => oracle.lints_for(&wrap(fx.code)),
        };
        if !before.ran {
            eprintln!(
                "score_bench: EXCLUDED {} — oracle did not run ({})",
                fx.id,
                before.error.clone().unwrap_or_default()
            );
            score.n_excluded += 1;
            continue;
        }
        if !before.compiles {
            eprintln!(
                "score_bench: EXCLUDED {} — fixture no longer compiles ({})",
                fx.id,
                before.error.clone().unwrap_or_default()
            );
            score.n_excluded += 1;
            continue;
        }
        let vocab_before: BTreeSet<String> = before.lints.intersection(&vocab).cloned().collect();
        let drifted = warn_label_drift(fx, &vocab_before);

        let (healed, edits, passes) = heal_fixpoint_cfg(fx.code, candidates, ovr);
        let (after_lints, output_ok, vocab_persist, created) =
            after_verdict(oracle, &healed, &vocab, &vocab_before, &before.lints);

        let outcome = FixtureOutcome {
            id: fx.id.to_string(),
            ran: true,
            vocab_before: vocab_before.clone(),
            all_before: before.lints.clone(),
            all_after: after_lints,
            vocab_persist: vocab_persist.clone(),
            created: created.clone(),
            edits,
            passes,
            parses: output_ok,
            drifted,
        };

        if vocab_before.is_empty() {
            acc.clean_total += 1;
            if !outcome.clean_violation() {
                acc.clean_ok += 1;
            }
        } else {
            acc.vocab_before_total += vocab_before.len();
            acc.resolved_total += outcome.resolved();
            for l in &vocab_persist {
                *acc.persist_counts.entry(l.clone()).or_default() += 1;
            }
            // Issue 089 T7 — the shaped verdict (healable fixtures only: a
            // clean fixture has no target to have cleared).
            acc.shaped_credit += outcome.shaped_verdict() * outcome.resolved() as f32;
            if outcome.is_shaped() {
                acc.n_shaped += 1;
                if vocab_before.len() == 1 {
                    let target = vocab_before.iter().next().cloned().unwrap_or_default();
                    *acc.half_counts.entry(target).or_default() += 1;
                }
            }
        }
        score.created_total += created.len();
        if edits > 0 {
            acc.edited += 1;
            if output_ok {
                acc.parses_ok += 1;
            }
        }
        score.edits_total += edits;
        score.outcomes.push(outcome);
    }

    score.n_ran = score.outcomes.len();
    score.n_clean = acc.clean_total;
    finalize_rates(&mut score, &acc);
    score
}

/// The loop accumulators for [`run_bench_with`] (one struct so the loop
/// + [`finalize_rates`] stay under the line budget).
#[derive(Default)]
struct RunAccum {
    vocab_before_total: usize,
    resolved_total: usize,
    clean_total: usize,
    clean_ok: usize,
    edited: usize,
    parses_ok: usize,
    persist_counts: BTreeMap<String, usize>,
    /// Issue 089 T7 — shaped-verdict counters.
    n_shaped: usize,
    shaped_credit: f32,
    half_counts: BTreeMap<String, u32>,
}

/// The frozen-label drift warning (the fresh verdict wins at bench time;
/// the print is the drift record). Returns whether drift was detected.
fn warn_label_drift(fx: &BenchFixture, vocab_before: &BTreeSet<String>) -> bool {
    let frozen: BTreeSet<String> = fx.expect_lints.iter().map(ToString::to_string).collect();
    let drifted = *vocab_before != frozen;
    if drifted {
        eprintln!(
            "score_bench: DRIFT {} — frozen {:?} vs fresh {:?} (fresh wins)",
            fx.id,
            frozen.iter().map(String::as_str).collect::<Vec<_>>(),
            vocab_before.iter().map(String::as_str).collect::<Vec<_>>(),
        );
    }
    drifted
}

/// Compute the aggregate rates from the loop accumulators.
// Count→ratio casts over fixture counts (< 10⁴) — far below f32's
// mantissa; allowed with cause.
#[allow(clippy::cast_precision_loss)]
fn finalize_rates(score: &mut BenchScore, accum: &RunAccum) {
    let RunAccum {
        vocab_before_total,
        resolved_total,
        clean_total,
        clean_ok,
        edited,
        parses_ok,
        persist_counts,
        n_shaped,
        shaped_credit,
        half_counts,
    } = accum;
    score.heal_rate = if *vocab_before_total > 0 {
        *resolved_total as f32 / *vocab_before_total as f32
    } else {
        0.0
    };
    score.n_shaped = *n_shaped;
    score.shaped_credit = *shaped_credit;
    score.shaped_rate = if *vocab_before_total > 0 {
        *shaped_credit / *vocab_before_total as f32
    } else {
        0.0
    };
    score.shaped_half_counts = half_counts.clone().into_iter().collect();
    score.created_rate = if *vocab_before_total > 0 {
        score.created_total as f32 / *vocab_before_total as f32
    } else {
        0.0
    };
    score.parse_safety = if *edited > 0 {
        *parses_ok as f32 / *edited as f32
    } else {
        1.0
    };
    score.decline_correctness = if *clean_total > 0 {
        *clean_ok as f32 / *clean_total as f32
    } else {
        1.0
    };
    let mut persist: Vec<(String, usize)> = persist_counts.clone().into_iter().collect();
    persist.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    score.persist = persist;
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Issue 089 T8: the per-rule outcome read ──────────────────────────

    /// Hand-build a single-target `FixtureOutcome` (the bench scores per
    /// lint NAME — `vocab_before` is a set, one name per fixture here).
    fn outcome(
        rule: &str,
        ran: bool,
        resolved: bool,
        parses: bool,
        created: usize,
        edits: usize,
    ) -> FixtureOutcome {
        let vocab_before: BTreeSet<String> = if rule.is_empty() {
            BTreeSet::new()
        } else {
            BTreeSet::from([rule.to_string()])
        };
        let vocab_persist = if parses && resolved {
            BTreeSet::new()
        } else {
            vocab_before.clone()
        };
        FixtureOutcome {
            id: format!("fx-{rule}"),
            ran,
            vocab_before: vocab_before.clone(),
            all_before: vocab_before.clone(),
            all_after: vocab_before.clone(),
            vocab_persist,
            created: BTreeSet::from_iter(
                (0..created).map(|i| format!("created_lint_{i}")),
            ),
            edits,
            passes: usize::from(edits > 0),
            parses,
            drifted: false,
        }
    }

    fn multi_target_outcome(rules: &[&str]) -> FixtureOutcome {
        FixtureOutcome {
            id: "fx-multi".to_string(),
            ran: true,
            vocab_before: BTreeSet::from_iter(rules.iter().map(ToString::to_string)),
            all_before: BTreeSet::new(),
            all_after: BTreeSet::new(),
            vocab_persist: BTreeSet::new(),
            created: BTreeSet::new(),
            edits: 3,
            passes: 1,
            parses: true,
            drifted: false,
        }
    }

    #[test]
    fn rule_outcomes_single_target_attribution_and_forks() {
        let score = BenchScore {
            outcomes: vec![
                // manual_map: one clean clear (shaped=no) + one shaped clear.
                outcome("manual_map", true, true, true, 0, 2),
                outcome("manual_map", true, true, true, 2, 3),
                // needless_return: still firing after the heal (persist).
                outcome("needless_return", true, false, true, 0, 1),
                // map_clone: healed output broke (broken fork).
                outcome("map_clone", true, false, false, 0, 2),
                // Excluded (oracle never ran) — counts nowhere.
                outcome("excluded_rule", false, true, true, 0, 0),
                // Clean fixture (the decline axis) — no rule.
                outcome("", true, false, true, 0, 0),
                // Multi-target composites — skipped loud, never guessed.
                multi_target_outcome(&["manual_map", "map_clone"]),
                multi_target_outcome(&["a", "b", "c"]),
            ],
            ..BenchScore::default()
        };
        let read = rule_outcomes(&score);
        assert_eq!(read.skipped_multi_target, 2);
        assert_eq!(read.rows.len(), 3, "rules: {:?}", read.rows);

        // Deterministic order: rule ascending.
        let names: Vec<&str> = read.rows.iter().map(|r| r.rule.as_str()).collect();
        assert_eq!(names, ["manual_map", "map_clone", "needless_return"]);

        let mm = &read.rows[0];
        assert_eq!((mm.fixtures, mm.resolved, mm.persist, mm.broken), (2, 2, 0, 0));
        assert_eq!((mm.created, mm.edits, mm.shaped), (2, 5, 1));

        let nr = &read.rows[2];
        assert_eq!((nr.fixtures, nr.resolved, nr.persist, nr.broken), (1, 0, 1, 0));

        let mc = &read.rows[1];
        assert_eq!((mc.fixtures, mc.resolved, mc.persist, mc.broken), (1, 0, 0, 1));

        // The fork invariant: fixtures == resolved + persist + broken.
        for r in &read.rows {
            assert_eq!(r.fixtures, r.resolved + r.persist + r.broken, "{}", r.rule);
        }
    }

    // ── Issue 089 T6: the stratified read ─────────────────────────────────

    #[test]
    fn provenance_class_maps_the_real_families() {
        use super::ood::provenance_family;
        // Classify the FAMILY (the prefix), never the full string.
        assert_eq!(
            provenance_class(provenance_family(
                "katgpt-rs@9e0e6a57:bench_ptrm_width_scaling.rs",
            )),
            ProvenanceClass::Repo
        );
        assert_eq!(
            provenance_class(provenance_family(
                "riir-train@a1197e06~1:crates/riir-train-engine/src/svr.rs",
            )),
            ProvenanceClass::Repo
        );
        assert_eq!(
            provenance_class(provenance_family(
                "authored-composite:T3.5.5:return_plus_identity_op",
            )),
            ProvenanceClass::Authored
        );
        assert_eq!(
            provenance_class(provenance_family("construction:gen6:k2:match_bool+use_self")),
            ProvenanceClass::Construction
        );
        assert_eq!(provenance_class("mystery"), ProvenanceClass::Other);
    }

    /// A v6 fixture — id on the raw-span table — classifies REPO through the
    /// mining join even though its frozen bench provenance says
    /// `construction:gen6:…` (the mislabel guard: the frozen tag is
    /// freeze-gate metadata, the raw table is mining truth).
    #[test]
    fn v6_fixtures_classify_repo_through_the_mining_join() {
        let v6 = super::generations::generation_v6();
        let fx = v6
            .iter()
            .find(|f| f.id == "gen6-svr-fn3-000")
            .expect("v6 fixture present");
        assert!(fx.provenance.starts_with("construction:gen6"));
        let family = super::ood::ood_set_family(fx);
        assert_eq!(family, "riir-train@a1197e06~1");
        assert_eq!(provenance_class(family), ProvenanceClass::Repo);
    }

    /// The stratified read's arithmetic + ordering, over synthetic fixtures
    /// covering every class + the excluded/clean/unattributed edges.
    #[test]
    fn stratify_splits_classes_and_orders_canonically() {
        // Exact f32 equality on heal_rate is a pin (0.0 / 0.5 / 1.0 are the
        // contract values); the near-equality asserts use 1e-6.
        #![allow(clippy::float_cmp)]
        let fixtures = vec![
            BenchFixture {
                id: "repo-a",
                provenance: "katgpt-rs@9e0e6a57:a.rs",
                code: "",
                expect_lints: &[],
                clean: false,
            },
            BenchFixture {
                id: "repo-b",
                provenance: "riir-chain@7421245a~1:b.rs",
                code: "",
                expect_lints: &[],
                clean: false,
            },
            BenchFixture {
                id: "authored-a",
                provenance: "authored-composite:T3.5.5:x",
                code: "",
                expect_lints: &[],
                clean: false,
            },
            BenchFixture {
                id: "construction-a",
                provenance: "construction:gen2:k2:a+b",
                code: "",
                expect_lints: &[],
                clean: false,
            },
        ];
        let mk = |id: &str, ran: bool, before: &[&str], persist: &[&str], created: &[&str]| {
            FixtureOutcome {
                id: id.to_string(),
                ran,
                vocab_before: before
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
                all_before: BTreeSet::new(),
                all_after: BTreeSet::new(),
                vocab_persist: persist
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
                created: created
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
                edits: 0,
                passes: 0,
                parses: true,
                drifted: false,
            }
        };
        let score = BenchScore {
            outcomes: vec![
                // repo-a: 2 findings, 1 resolved, 1 created.
                mk("repo-a", true, &["a", "b"], &["b"], &["new"]),
                // repo-b: CLEAN (0 findings, ran).
                mk("repo-b", true, &[], &[], &[]),
                // authored-a: 1 finding, resolved.
                mk("authored-a", true, &["a"], &[], &[]),
                // construction-a: EXCLUDED (ran=false — counts nowhere).
                mk("construction-a", false, &["a"], &["a"], &[]),
                // An outcome with no fixture (a defensive-count case → other).
                mk("orphan", true, &["z"], &["z"], &[]),
            ],
            ..BenchScore::default()
        };
        let strata = stratify(&fixtures, &score);
        let labels: Vec<&str> = strata.iter().map(|s| s.class.as_str()).collect();
        // Canonical order; construction omitted (0 ran).
        assert_eq!(labels, ["repo", "authored", "other"]);
        let repo = &strata[0];
        assert_eq!(repo.n_ran, 2);
        assert_eq!(repo.n_healable, 1);
        assert_eq!(repo.n_clean, 1);
        assert_eq!(repo.vocab_before, 2);
        assert_eq!(repo.resolved, 1);
        assert_eq!(repo.created, 1);
        assert!((repo.heal_rate - 0.5).abs() < 1e-6);
        let authored = &strata[1];
        assert_eq!((authored.n_ran, authored.vocab_before, authored.resolved), (1, 1, 1));
        assert!((authored.heal_rate - 1.0).abs() < 1e-6);
        let other = &strata[2];
        assert_eq!(other.n_ran, 1);
        assert_eq!(other.heal_rate, 0.0); // 1 finding, 0 resolved
    }

    // ── Issue 089 T7: the shaped verdict ─────────────────────────────────

    #[test]
    fn shaped_verdict_prices_the_three_outcomes() {
        let mk = |before: &[&str], persist: &[&str], created: &[&str], parses: bool| {
            FixtureOutcome {
                id: "x".to_string(),
                ran: true,
                vocab_before: before.iter().map(ToString::to_string).collect(),
                all_before: BTreeSet::new(),
                all_after: BTreeSet::new(),
                vocab_persist: persist.iter().map(ToString::to_string).collect(),
                created: created.iter().map(ToString::to_string).collect(),
                edits: 1,
                passes: 1,
                parses,
                drifted: false,
            }
        };
        // Clean clear → 1.0.
        let clean_clear = mk(&["a"], &[], &[], true);
        assert!((clean_clear.shaped_verdict() - 1.0).abs() < 1e-6);
        assert!(!clean_clear.is_shaped());
        // Target cleared ∧ new diags → exactly SHAPED_VERDICT (0.5).
        let shaped = mk(&["a"], &[], &["redundant_closure"], true);
        assert!((shaped.shaped_verdict() - SHAPED_VERDICT).abs() < 1e-6);
        assert!((SHAPED_VERDICT - 0.5).abs() < 1e-6);
        assert!(shaped.is_shaped());
        // Persist → 0.0 (nothing cleared).
        let persist = mk(&["a"], &["a"], &[], true);
        assert!((persist.shaped_verdict() - 0.0).abs() < 1e-6);
        // Broken output → 0.0 even though vocab_persist is empty (the
        // after_verdict contract already scores breakage as nothing
        // resolved — the verdict must not re-credit it).
        let broken = mk(&["a"], &[], &["x"], false);
        assert!((broken.shaped_verdict() - 0.0).abs() < 1e-6);
        // Clean fixture (declined, no target) → 0.0 — a shaped verdict needs
        // a target to have cleared.
        let declined = mk(&[], &[], &[], true);
        assert!((declined.shaped_verdict() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn shaped_half_counts_attribute_single_target_fixtures_only() {
        // The attribution law: a shaped fixture with EXACTLY ONE vocab lint
        // credits that rule; a multi-target shaped fixture counts in the
        // rates but NOT in half_counts (recorded, not guessed). Exercised
        // through the loop accumulator shape — the arithmetic is
        // `run_bench_with`'s; here we pin the classification inputs.
        let single = FixtureOutcome {
            id: "one".to_string(),
            ran: true,
            vocab_before: ["manual_map".to_string()].into_iter().collect(),
            all_before: BTreeSet::new(),
            all_after: BTreeSet::new(),
            vocab_persist: BTreeSet::new(),
            created: ["redundant_closure".to_string()].into_iter().collect(),
            edits: 1,
            passes: 1,
            parses: true,
            drifted: false,
        };
        assert!(single.is_shaped());
        assert_eq!(single.vocab_before.len(), 1);
        let multi = FixtureOutcome {
            vocab_before: ["a".to_string(), "b".to_string()].into_iter().collect(),
            created: ["c".to_string()].into_iter().collect(),
            ..single.clone()
        };
        assert!(multi.is_shaped());
        assert_ne!(multi.vocab_before.len(), 1);
    }

    #[test]
    fn heal_fixpoint_fixes_a_known_lint() {
        // uninlined_format_args (pedantic) — the highest-frequency class in
        // the sweep record.
        let code = "pub fn greet(name: &str) -> String {\n    format!(\"{}\", name)\n}\n";
        let (healed, edits, _passes) = heal_fixpoint(code);
        assert!(
            edits >= 1,
            "at least the format-arg edit, got {edits}: {healed}"
        );
        assert!(healed.contains("{name}"), "inlined: {healed}");
    }

    #[test]
    fn heal_fixpoint_leaves_clean_code_alone() {
        let code = "pub fn greet(name: &str) -> String {\n    format!(\"{name}\")\n}\n";
        let (healed, edits, _) = heal_fixpoint(code);
        assert_eq!(edits, 0);
        assert_eq!(healed, code);
    }

    #[test]
    fn wrap_prepends_allow_pragmas() {
        let w = wrap("fn f() {}");
        assert!(w.starts_with("#![allow(dead_code)]"));
        assert!(w.contains("fn f() {}"));
    }

    #[test]
    fn vocabulary_is_nonempty_and_bare() {
        let v = vocabulary();
        assert!(!v.is_empty());
        assert!(v.contains("uninlined_format_args"));
        assert!(!v.iter().any(|l| l.contains("clippy::")));
    }

    /// Per-fixture miss-map triage (run with `--nocapture`):
    /// `cargo test --features score_bench --lib score_bench::tests::per_fixture_diagnostics -- --nocapture --ignored`
    ///
    /// Ignored by default — it shells out to real cargo clippy for every
    /// fixture (the full bench run). The standing workflow for curve steps:
    /// read the aggregate map from `--score-bench`, then this test for the
    /// per-fixture detail (which fixture creates/persists which lint).
    #[test]
    #[ignore = "shells out to real cargo clippy for all 34 fixtures (the full bench)"]
    fn per_fixture_diagnostics() {
        let oracle = draft_oracle();
        assert!(oracle.is_available(), "cargo not on PATH");
        let score = run_bench(&oracle, &fixtures::fixtures());
        for o in &score.outcomes {
            let created: Vec<&str> = o.created.iter().map(String::as_str).collect();
            let persist: Vec<&str> = o.vocab_persist.iter().map(String::as_str).collect();
            if created.is_empty() && persist.is_empty() {
                continue;
            }
            println!("── {} (edits={}, passes={})", o.id, o.edits, o.passes);
            if !created.is_empty() {
                println!("   CREATED : {created:?}");
            }
            if !persist.is_empty() {
                println!("   PERSIST : {persist:?}");
            }
        }
    }
}

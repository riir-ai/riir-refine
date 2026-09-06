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
//! recorded), heals it with the SAME bounded-fixpoint loop as `cargo heal
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

pub mod fixtures;
pub mod generations;
pub mod report;

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::domains::clippy_lints::proposers::FIXABLE_LINTS;
use crate::domains::rust_perf::proposers::FIXABLE_RULES;
use crate::draft::clippy_oracle::ClippyOracle;
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
            edits.extend(crate::domains::rust_perf::proposers::collect_edits(
                rule, &current, &parsed,
            ));
        }
        for lint in FIXABLE_LINTS {
            edits.extend(crate::domains::clippy_lints::proposers::collect_edits(
                lint, &current, &parsed,
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
pub fn run_bench_with(
    oracle: &ClippyOracle,
    fixture_list: &[BenchFixture],
    candidates: &[(&str, &str)],
) -> BenchScore {
    let vocab = vocabulary();
    let mut score = BenchScore {
        n_total: fixture_list.len(),
        vocab_size: vocab.len(),
        ..BenchScore::default()
    };
    let mut vocab_before_total = 0usize;
    let mut resolved_total = 0usize;
    let mut persist_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut clean_total = 0usize;
    let mut clean_ok = 0usize;
    let mut edited = 0usize;
    let mut parses_ok = 0usize;

    for fx in fixture_list {
        let before = oracle.lints_for(&wrap(fx.code));
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
        let vocab_before: BTreeSet<String> =
            before.lints.intersection(&vocab).cloned().collect();
        let frozen: BTreeSet<String> = fx.expect_lints.iter().map(ToString::to_string).collect();
        let drifted = vocab_before != frozen;
        if drifted {
            eprintln!(
                "score_bench: DRIFT {} — frozen {:?} vs fresh {:?} (fresh wins)",
                fx.id,
                frozen.iter().map(String::as_str).collect::<Vec<_>>(),
                vocab_before.iter().map(String::as_str).collect::<Vec<_>>(),
            );
        }

        let (healed, edits, passes) = heal_fixpoint_with(fx.code, candidates);
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
            clean_total += 1;
            if !outcome.clean_violation() {
                clean_ok += 1;
            }
        } else {
            vocab_before_total += vocab_before.len();
            resolved_total += outcome.resolved();
            for l in &vocab_persist {
                *persist_counts.entry(l.clone()).or_default() += 1;
            }
        }
        score.created_total += created.len();
        if edits > 0 {
            edited += 1;
            if output_ok {
                parses_ok += 1;
            }
        }
        score.edits_total += edits;
        score.outcomes.push(outcome);
    }

    score.n_ran = score.outcomes.len();
    score.n_clean = clean_total;
    finalize_rates(
        &mut score,
        &Accum {
            vocab_before_total,
            resolved_total,
            clean_total,
            clean_ok,
            edited,
            parses_ok,
            persist_counts,
        },
    );
    score
}

/// Loop accumulators for [`run_bench`] (kept as one struct so the
/// finalize step + the loop stay under the line budget).
struct Accum {
    vocab_before_total: usize,
    resolved_total: usize,
    clean_total: usize,
    clean_ok: usize,
    edited: usize,
    parses_ok: usize,
    persist_counts: BTreeMap<String, usize>,
}

/// Compute the aggregate rates from the loop accumulators.
// Count→ratio casts over fixture counts (< 10⁴) — far below f32's
// mantissa; allowed with cause.
#[allow(clippy::cast_precision_loss)]
fn finalize_rates(score: &mut BenchScore, accum: &Accum) {
    let Accum {
        vocab_before_total,
        resolved_total,
        clean_total,
        clean_ok,
        edited,
        parses_ok,
        persist_counts,
    } = accum;
    score.heal_rate = if *vocab_before_total > 0 {
        *resolved_total as f32 / *vocab_before_total as f32
    } else {
        0.0
    };
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

    #[test]
    fn heal_fixpoint_fixes_a_known_lint() {
        // uninlined_format_args (pedantic) — the highest-frequency class in
        // the sweep record.
        let code = "pub fn greet(name: &str) -> String {\n    format!(\"{}\", name)\n}\n";
        let (healed, edits, _passes) = heal_fixpoint(code);
        assert!(edits >= 1, "at least the format-arg edit, got {edits}: {healed}");
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

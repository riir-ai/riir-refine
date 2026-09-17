//! Issue 122 `PoC` — the `PowerPlay` frontier miner: a modelless generator that
//! constructs the simplest code fixtures the current fix-rule corpus cannot
//! yet repair, verifies candidates with the real clippy oracle only, and
//! commits admissible rows at the capability frontier (unsolved-before /
//! solved-after / prior-rows-green / exactly-once).
//!
//! ## The pinned claim (novelty-bearing form, from the issue)
//!
//! A **modelless generator** (non-learning, zero LLM, deterministic given a
//! seed) that constructs **the simplest code fixtures the current fix-rule
//! corpus cannot yet repair**, verifies candidates with the **compile/clippy
//! oracle only** (no LLM in the verification loop), and grows the corpus at
//! the capability frontier — `PowerPlay`'s acceptance invariant applied to the
//! healer's own rule corpus. Prior-art deltas: (i) the generator has no LLM
//! and does not learn (vs AZR/SSR/Voyager); (ii) the substrate grown is a
//! modelless rule corpus, not weights or agent code (vs SSR/AZR/DGM);
//! (iii) the frontier is defined by the corpus's own repair failures, not a
//! learning-progress reward (vs AZR); (iv) the PowerPlay→code-corpus lineage
//! gap stands 2015-2026.
//!
//! ## Two admission modes
//!
//! - [`Mode::HeldOut`] — the corpus VIEW disables one COVERED rule
//!   (via [`ComponentOverride`], the Issue-108 necessity seam); generated
//!   fixtures that only that rule heals are unsolved-by-the-view; re-enabling
//!   the rule IS the solution. This exercises the FULL acceptance invariant
//!   loop (unsolved-before → solved-after → regression-green) mechanically,
//!   with zero human mining, and produces **band evidence** for the
//!   Issue-089 admission gate (the solving rule gains f=1 then r=1 →
//!   p̂=0.5 → [`BandClass::InBand`], the synthetic/construction feed —
//!   ε-priced, never rating evidence).
//! - [`Mode::Frontier`] — families target lints OUTSIDE the compiled fix set
//!   (`FIXABLE_LINTS ∪ VERIFY_LINTS`); clippy fires, nothing heals → a
//!   genuine **mining lead** row (the self-play twin of `--mine`). Clause 3
//!   (solved-after) is PENDING for these rows by construction: no rule
//!   exists yet; a mining batch that lands the rule flips the row to solved
//!   on the next pass.
//!
//! ## Admissibility invariant (per generated row — the issue's four clauses)
//!
//! 1. **Oracle-declared defect:** the broken form compiles AND real clippy
//!    fires the family's target lint on it; the SEED is clean (0 clippy
//!    lints — measured, never asserted; the `corpus_interacting` lesson).
//! 2. **Unsolved-before:** healing under the view leaves the target lint
//!    firing — verified by a SECOND oracle call on the healed output (the
//!    oracle-only discipline; never a text-shape heuristic).
//! 3. **Solved-after + regression:** with the solution active the target
//!    lint clears, AND every stock corpus row heals byte-identically with
//!    the solution active (a solution that re-breaks or perturbs a prior
//!    row is REFUSED — the adversarial arm pins it).
//! 4. **Exactly-once:** BLAKE3 of the fixture source, checked against the
//!    committed row store on load.
//!
//! ## K(c) minimality (simplest-first, MDL tie-break)
//!
//! Candidates enumerate at ops=1 first; within a family the canonical order
//! is `(ops, BLAKE3)` and the FIRST admissible candidate commits — a family
//! with an ops=1 admission never takes ops=2 in the same pass.
//!
//! ## Substrate table (substrate-first, Mode 1 — see Plan 145)
//!
//! | concern | consumed substrate |
//! |---|---|
//! | heal loop + held-out view | [`score_bench::heal_fixpoint_cfg`] + [`ComponentOverride`] |
//! | solution lane (a new entry) | `heal_fixpoint_cfg`'s `candidates` textual lane |
//! | real-clippy verdicts | [`ClippyOracle`] / [`OracleVerdict`] (`draft::clippy_oracle`) |
//! | band arithmetic | [`frontier_report::summarize`] / [`BandClass`] |
//! | provenance pricing | [`frontier_pairs::PROVENANCE`] (construction, ε=0.05) |
//! | stock regression rows | [`build_seed_corpus`] (read-only — **never mutated**; the freeze-table law) |
//!
//! Nothing here touches `build_seed_corpus()`'s entries: generated fixtures
//! are DERIVED at run time from this module's own tables and land in
//! `.benchmarks/frontier_miner_rows.jsonl`, never in the committed corpus —
//! the gen2/gen3 freeze tables are unmoved by construction (pinned by test).
//!
//! ## Two-tier verification (the freeze-gate / `interacting_gate` precedent)
//!
//! The admission CORE is pure over an injected oracle probe (`&dyn Fn(&str)
//! -> OracleVerdict`) — hermetic lib tests drive it with mock verdicts and
//! the compiled proposers. The real-clippy e2e lives in
//! `examples/frontier_miner.rs` (the authority for admission claims; spawns
//! `ClippyOracle` on temp crates).

use std::collections::BTreeSet;

use crate::draft::clippy_oracle::OracleVerdict;
use crate::frontier_pairs::PROVENANCE;
use crate::frontier_report;
use crate::score_bench::{heal_fixpoint_cfg, ComponentOverride};

/// Where committed miner rows live (append-only; BLAKE3-deduped on load).
pub const MINER_ROWS_JSONL: &str = ".benchmarks/frontier_miner_rows.jsonl";

/// The held-out rule for [`Mode::HeldOut`] — the seed-six classic whose
/// proposer is the most battle-tested in the corpus (its corpus fixture is
/// entry #1 and the proposer's rewrite is pinned by `all_buggy_codes_fire_
/// their_lint`).
pub const HELD_OUT_RULE: &str = "needless_return";

// ─────────────────────────────────────────────────────────────────────────
// Section 1 — families: clean seeds + 1-op mutations (the corpus_gen pattern:
// table-driven, parametric, NO RNG — byte-stable per build)
// ─────────────────────────────────────────────────────────────────────────

/// One mutation family: a target lint + a parametric table of
/// (clean seed, broken form) pairs, all produced by ONE structural mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Family {
    /// Stable family id (row-store key).
    pub id: &'static str,
    /// The clippy lint the broken form is designed to trip.
    pub target_lint: &'static str,
    /// `true` when the target lint is in the compiled fix set (the `HeldOut`
    /// arm's population); `false` = a genuine frontier family.
    pub covered: bool,
}

/// The family inventory. Frontier targets verified ABSENT from
/// `FIXABLE_LINTS ∪ VERIFY_LINTS` at authoring time (clippy 1.98 era):
/// `let_and_return`, `ptr_arg`, `needless_borrow`, `redundant_slicing`,
/// `map_flatten` — all default-firing (style/complexity), so they surface
/// on a plain `cargo clippy`, not only under pedantic/nursery.
#[must_use]
pub fn families() -> Vec<Family> {
    vec![
        Family { id: "needless_return-heldout", target_lint: "needless_return", covered: true },
        Family { id: "let_and_return", target_lint: "let_and_return", covered: false },
        Family { id: "ptr_arg", target_lint: "ptr_arg", covered: false },
        Family { id: "needless_borrow", target_lint: "needless_borrow", covered: false },
        Family { id: "redundant_slicing", target_lint: "redundant_slicing", covered: false },
        Family { id: "map_flatten", target_lint: "map_flatten", covered: false },
    ]
}

/// One generated candidate: the broken fixture plus its provenance.
/// `ops` is K(c) — the mutation-op count (every table entry is a 1-op
/// structural mutation; composition for ops≥2 is future work, recorded in
/// the issue as dependent-mutation composition).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub family: &'static str,
    pub target_lint: &'static str,
    /// The admission mode this family serves (set at generate-time from the
    /// family's `covered` flag — never re-derived by string matching).
    pub mode: Mode,
    pub ops: u32,
    /// The clean, clippy-clean seed (the known inverse — construction truth).
    pub seed: String,
    /// The broken form the oracle judges.
    pub broken: String,
    /// BLAKE3 of `broken` (the content address — exactly-once key).
    pub blake3: String,
}

/// Deterministically generate the candidate set for the requested modes.
///
/// Pure: a function of `modes` alone — no clock, no env, no RNG. Canonical
/// order: `(family id asc, blake3 asc)`; MDL simplest-first is the per-family
/// pass order in [`admit`] (ops asc, then blake3).
#[must_use]
pub fn generate(modes: &[Mode]) -> Vec<Candidate> {
    let mut out = Vec::new();
    let want_heldout = modes.contains(&Mode::HeldOut);
    let want_frontier = modes.contains(&Mode::Frontier);
    for fam in families() {
        let wanted = if fam.covered { want_heldout } else { want_frontier };
        if !wanted {
            continue;
        }
        for (seed, broken) in family_cases(fam) {
            out.push(Candidate {
                family: fam.id,
                target_lint: fam.target_lint,
                mode: if fam.covered { Mode::HeldOut } else { Mode::Frontier },
                ops: 1,
                seed: seed.to_string(),
                broken: broken.to_string(),
                blake3: blake3_hex(broken),
            });
        }
    }
    out.sort_by(|a, b| a.family.cmp(b.family).then_with(|| a.blake3.cmp(&b.blake3)));
    out.shrink_to_fit();
    out
}

/// The parametric case tables (seed, broken) per family. Idents vary per
/// case so BLAKE3 distinguishes them (the exactly-once set stays honest).
fn family_cases(fam: Family) -> Vec<(&'static str, &'static str)> {
    match fam.id {
        // 1-op: wrap the tail expression in `return …;`. Seeds are
        // deliberately NON-const-able (format!/iterator/any shapes) — the
        // oracle runs all+pedantic+nursery and `missing_const_for_fn`
        // (nursery) fires on every trivially-const seed, which would be a
        // dirty seed under the full oracle (measured, the first e2e run).
        "needless_return-heldout" => vec![
            ("fn joined(v: &[i32]) -> String { format!(\"{v:?}\") }", "fn joined(v: &[i32]) -> String { return format!(\"{v:?}\"); }"),
            ("fn label(id: u32) -> String { format!(\"item-{id}\") }", "fn label(id: u32) -> String { return format!(\"item-{id}\"); }"),
            ("fn has_neg(v: &[i32]) -> bool { v.iter().any(|x| *x < 0) }", "fn has_neg(v: &[i32]) -> bool { return v.iter().any(|x| *x < 0); }"),
        ],
        // 1-op: bind the tail expression to a trailing let and return the name.
        "let_and_return" => vec![
            ("fn shout(s: &str) -> String { s.to_uppercase() }", "fn shout(s: &str) -> String { let out = s.to_uppercase(); out }"),
            ("fn total(v: &[i32]) -> i32 { v.iter().sum() }", "fn total(v: &[i32]) -> i32 { let sum = v.iter().sum(); sum }"),
            ("fn slug(s: &str) -> String { s.replace(' ', \"-\") }", "fn slug(s: &str) -> String { let made = s.replace(' ', \"-\"); made }"),
        ],
        // 1-op: narrow `&[T]` to `&Vec<T>` in the signature.
        "ptr_arg" => vec![
            ("fn total(v: &[i32]) -> i32 { v.iter().sum() }", "fn total(v: &Vec<i32>) -> i32 { v.iter().sum() }"),
            ("fn first_len(rows: &[String]) -> usize { rows.first().map_or(0, String::len) }", "fn first_len(rows: &Vec<String>) -> usize { rows.first().map_or(0, String::len) }"),
            ("fn any_zero(v: &[u8]) -> bool { v.contains(&0) }", "fn any_zero(v: &Vec<u8>) -> bool { v.contains(&0) }"),
        ],
        // 1-op: wrap the receiver in a redundant borrow (non-const method
        // receivers — the same missing_const_for_fn nursery discipline).
        "needless_borrow" => vec![
            ("fn upper(s: &str) -> String { s.to_uppercase() }", "fn upper(s: &str) -> String { (&s).to_uppercase() }"),
            ("fn digits(s: &str) -> usize { s.matches(char::is_numeric).count() }", "fn digits(s: &str) -> usize { (&s).matches(char::is_numeric).count() }"),
            ("fn head_len(s: &str) -> usize { s.trim().len() }", "fn head_len(s: &str) -> usize { (&s.trim()).len() }"),
        ],
        // 1-op: pass a redundant full-slice `&s[..]` to a `&str` param.
        "redundant_slicing" => vec![
            ("fn first_char(s: &str) -> char { s.chars().next().unwrap_or(' ') }\nfn pick(s: &str) -> char { first_char(s) }", "fn first_char(s: &str) -> char { s.chars().next().unwrap_or(' ') }\nfn pick(s: &str) -> char { first_char(&s[..]) }"),
            ("fn tail(s: &str) -> &str { &s[1..] }\nfn clip(s: &str) -> &str { tail(s) }", "fn tail(s: &str) -> &str { &s[1..] }\nfn clip(s: &str) -> &str { tail(&s[..]) }"),
        ],
        // 1-op: `.map(f).flatten()` → the broken form is the un-fused pair
        // (seed carries the fused `flat_map`).
        "map_flatten" => vec![
            ("fn word_lines(xs: &[String]) -> Vec<&str> { xs.iter().flat_map(|x| x.split(' ')).collect() }", "fn word_lines(xs: &[String]) -> Vec<&str> { xs.iter().map(|x| x.split(' ')).flatten().collect() }"),
            ("fn chunks(rows: &[String]) -> Vec<&str> { rows.iter().flat_map(|r| r.split(',')).collect() }", "fn chunks(rows: &[String]) -> Vec<&str> { rows.iter().map(|r| r.split(',')).flatten().collect() }"),
        ],
        _ => vec![],
    }
}

/// BLAKE3 of the fixture source, 16-hex (the content-address convention).
#[must_use]
pub fn blake3_hex(src: &str) -> String {
    let mut h = blake3::Hasher::new();
    h.update(src.as_bytes());
    let hex = h.finalize().to_hex();
    hex.as_str()[..16].to_string()
}

/// BLAKE3 over the whole derived candidate set (the G4 reproducibility
/// pin: same inputs → byte-identical derived corpus, across machines).
#[must_use]
pub fn derived_set_blake3(candidates: &[Candidate]) -> String {
    let mut h = blake3::Hasher::new();
    for c in candidates {
        h.update(c.family.as_bytes());
        h.update(c.blake3.as_bytes());
        h.update(c.broken.as_bytes());
    }
    let hex = h.finalize().to_hex();
    hex.as_str()[..16].to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// Section 2 — mode, view, solution
// ─────────────────────────────────────────────────────────────────────────

/// Which admission mode a pass runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Corpus view holds out [`HELD_OUT_RULE`]; the re-enable is the solution.
    HeldOut,
    /// Families target lints outside the compiled fix set (mining leads).
    Frontier,
}

/// The corpus VIEW the unsolved-before check heals under (the Issue-108
/// intervention seam, consumed as a read-only lens).
#[derive(Clone, Debug, Default)]
pub struct CorpusView {
    /// Rules whose dispatch slot is emptied for this view.
    pub disabled: BTreeSet<String>,
}

impl CorpusView {
    /// The `HeldOut` view: [`HELD_OUT_RULE`] disabled.
    #[must_use]
    pub fn held_out() -> Self {
        Self { disabled: BTreeSet::from([HELD_OUT_RULE.to_string()]) }
    }

    fn override_for(&self) -> ComponentOverride {
        let mut ovr = ComponentOverride::default();
        for rule in &self.disabled {
            ovr.disabled.insert(rule.clone());
        }
        ovr
    }
}

/// A solution lane: how the corpus grows (or re-enables) to clear an
/// admitted fixture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Solution {
    /// Re-enable a held-out rule (`HeldOut` mode's mechanical solution).
    Reenable(String),
    /// A new corpus entry's first fixture as a textual (buggy, fixed) pair —
    /// the `heal_fixpoint_cfg` candidate lane, exactly what a mining batch
    /// lands first.
    Entry { buggy: String, fixed: String },
}

impl Solution {
    /// The textual candidate lane this solution contributes (borrowed —
    /// `heal_fixpoint_cfg`'s `&[(&str, &str)]` shape).
    fn candidates(&self) -> Vec<(&str, &str)> {
        match self {
            Self::Reenable(_) => Vec::new(),
            Self::Entry { buggy, fixed } => vec![(buggy.as_str(), fixed.as_str())],
        }
    }

    /// The rule credited with the solve (band-evidence attribution).
    #[must_use]
    pub fn solving_rule(&self) -> String {
        match self {
            Self::Reenable(rule) => rule.clone(),
            Self::Entry { buggy, .. } => format!("entry:{}", blake3_hex(buggy)),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Section 3 — admission (the invariant core; oracle-injected, pure)
// ─────────────────────────────────────────────────────────────────────────

/// Why a candidate was NOT admitted (every rejection is evidence — the pass
/// reports them; a green-zero pass is a loud finding, not silence).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejection {
    /// The oracle could not run / the broken form does not compile.
    BrokenNotVerifiable { family: &'static str, why: String },
    /// The broken form does not fire the family's target lint (family drift
    /// — clippy moved; the row is reported, never silently skipped).
    TargetLintNotFired { family: &'static str, saw: Vec<String> },
    /// The SEED itself trips clippy (a broken family table — measured, not
    /// asserted; the `corpus_interacting` lesson).
    SeedNotClean { family: &'static str, saw: Vec<String> },
    /// The view's corpus HEALED it — not at the frontier (good news: the
    /// corpus is broader than the family table assumed).
    HealedByView { family: &'static str },
    /// BLAKE3 already committed (exactly-once held).
    Duplicate { family: &'static str, blake3: String },
    /// (`HeldOut`) The solution failed to clear the target lint.
    SolutionDidNotClear { family: &'static str },
    /// (`HeldOut`) The solution perturbed a stock row's heal output — REFUSED
    /// (the adversarial clause: a solution that re-breaks a prior row never
    /// commits).
    RegressionRefused { family: &'static str, perturbed: Vec<String> },
}

/// One admissible row: the committed record (jsonl shape).
#[derive(Clone, Debug, PartialEq)]
pub struct AdmittedRow {
    pub family: &'static str,
    pub target_lint: &'static str,
    pub ops: u32,
    pub blake3: String,
    pub mode: Mode,
    /// The rule credited with the solve (`HeldOut`) — `None` while the row is
    /// a pending mining lead (Frontier).
    pub solving_rule: Option<String>,
    /// Every clippy lint that fired on the broken form (context; the
    /// steering axis for mining).
    pub lints_before: Vec<String>,
    /// Every clippy lint still firing on the view-healed output.
    pub lints_after: Vec<String>,
    pub seed_blake3: String,
}

/// One pass's outcome.
#[derive(Clone, Debug, Default)]
pub struct PassOutcome {
    pub admitted: Vec<AdmittedRow>,
    pub rejections: Vec<Rejection>,
}

/// The oracle probe: real `ClippyOracle::lints_for` in the e2e example,
/// mock verdicts in hermetic tests. Takes the code to judge.
pub type OracleProbe<'a> = &'a dyn Fn(&str) -> OracleVerdict;

/// Admit candidates under the invariant. For [`Mode::HeldOut`], pass the
/// matching [`Solution`] — clauses 2+3 run; a row commits only when the
/// full loop holds. For [`Mode::Frontier`], `solution: None` — clauses
/// 1/2/4 run and clause 3 records PENDING (`solving_rule: None`).
///
/// Per-family K-minimality: candidates are processed in `(ops, blake3)`
/// order and a family stops at its first admission in the pass — and at
/// its first PRIOR admission (`known.families`) across passes.
#[must_use]
pub fn admit(
    candidates: &[Candidate],
    view: &CorpusView,
    solution: Option<&Solution>,
    known: &KnownSet,
    probe: OracleProbe,
) -> PassOutcome {
    let mut out = PassOutcome::default();
    let mut admitted_families: BTreeSet<String> = BTreeSet::new();
    // Per-family canonical order: ops asc, then blake3 (MDL tie-break).
    let mut ordered: Vec<&Candidate> = candidates.iter().collect();
    ordered.sort_by(|a, b| a.ops.cmp(&b.ops).then_with(|| a.blake3.cmp(&b.blake3)));

    for c in ordered {
        if admitted_families.contains(c.family) || known.families.contains(c.family) {
            continue; // K-minimality: first admissible in the family won.
        }
        if known.blake3s.contains(&c.blake3) {
            out.rejections.push(Rejection::Duplicate { family: c.family, blake3: c.blake3.clone() });
            continue;
        }
        // Clause 1a — the seed is clean (measured).
        let seed_verdict = probe(&c.seed);
        if !seed_verdict.ran || !seed_verdict.compiles || !seed_verdict.lints.is_empty() {
            out.rejections.push(Rejection::SeedNotClean {
                family: c.family,
                saw: seed_verdict.lints.iter().cloned().collect(),
            });
            continue;
        }
        // Clause 1b — the broken form compiles and fires the target lint.
        let broken_verdict = probe(&c.broken);
        if !broken_verdict.ran || !broken_verdict.compiles {
            out.rejections.push(Rejection::BrokenNotVerifiable {
                family: c.family,
                why: broken_verdict.error.clone().unwrap_or_else(|| "did not run".into()),
            });
            continue;
        }
        if !broken_verdict.lints.contains(c.target_lint) {
            out.rejections.push(Rejection::TargetLintNotFired {
                family: c.family,
                saw: broken_verdict.lints.iter().cloned().collect(),
            });
            continue;
        }
        // Clause 2 — unsolved-before: heal under the VIEW; the target lint
        // must still fire on the healed output (oracle-judged).
        let (healed, _edits, _passes) = heal_fixpoint_cfg(&c.broken, &[], &view.override_for());
        let healed_verdict = probe(&healed);
        if healed_verdict.ran
            && healed_verdict.compiles
            && !healed_verdict.lints.contains(c.target_lint)
        {
            out.rejections.push(Rejection::HealedByView { family: c.family });
            continue;
        }
        // Clause 3 — solved-after + regression (HeldOut mode only).
        let solving_rule = match (c.mode, solution) {
            (Mode::HeldOut, Some(sol)) => {
                let (_solved, cleared) = apply_solution(c, sol, probe);
                if !cleared {
                    out.rejections.push(Rejection::SolutionDidNotClear { family: c.family });
                    continue;
                }
                if let Some(perturbed) = regression_perturbations(sol) {
                    out.rejections.push(Rejection::RegressionRefused { family: c.family, perturbed });
                    continue;
                }
                Some(sol.solving_rule())
            }
            (Mode::HeldOut, None) => {
                // A HeldOut pass without its solution is a caller error —
                // loud, never a silent pending row.
                out.rejections.push(Rejection::SolutionDidNotClear { family: c.family });
                continue;
            }
            (Mode::Frontier, _) => None, // pending mining lead
        };
        out.admitted.push(AdmittedRow {
            family: c.family,
            target_lint: c.target_lint,
            ops: c.ops,
            blake3: c.blake3.clone(),
            mode: c.mode,
            solving_rule,
            lints_before: broken_verdict.lints.iter().cloned().collect(),
            lints_after: healed_verdict.lints.iter().cloned().collect(),
            seed_blake3: blake3_hex(&c.seed),
        });
        admitted_families.insert(c.family.to_string());
    }
    out
}

/// Clause 2's heal half as a standalone (the e2e's committed-row verify
/// path re-runs it): heal the fixture under a view, oracle-free.
#[must_use]
pub fn heal_under_view(c: &Candidate, view: &CorpusView) -> (String, usize, usize) {
    heal_fixpoint_cfg(&c.broken, &[], &view.override_for())
}

/// Clause 3a — heal with the solution active; `cleared` is oracle-judged
/// (the target lint no longer fires on the solution-healed output).
#[must_use]
pub fn apply_solution(c: &Candidate, solution: &Solution, probe: OracleProbe) -> (String, bool) {
    // A Reenable solution heals under the DEFAULT view (the rule's slot is
    // back); an Entry solution contributes its textual candidate lane on
    // the default view. Both are exactly what a mining batch's first
    // landing looks like to the pipeline.
    let sol_candidates = solution.candidates();
    let (healed, _edits, _passes) = heal_fixpoint_cfg(
        &c.broken,
        &sol_candidates,
        &ComponentOverride::default(),
    );
    let verdict = probe(&healed);
    let cleared = verdict.ran && verdict.compiles && !verdict.lints.contains(c.target_lint);
    (healed, cleared)
}

/// Clause 3b — the regression arm: every stock corpus row must heal
/// BYTE-IDENTICALLY with the solution active. Returns the perturbed rule
/// ids when the arm refuses (None = holds).
#[must_use]
pub fn regression_perturbations(solution: &Solution) -> Option<Vec<String>> {
    let sol_candidates = solution.candidates();
    let mut perturbed = Vec::new();
    for entry in &crate::build_seed_corpus().entries {
        let (plain, _, _) = heal_fixpoint_cfg(&entry.buggy_code, &[], &ComponentOverride::default());
        let (with_sol, _, _) =
            heal_fixpoint_cfg(&entry.buggy_code, &sol_candidates, &ComponentOverride::default());
        if plain != with_sol {
            perturbed.push(entry.lint_name.clone());
        }
    }
    if perturbed.is_empty() { None } else { Some(perturbed) }
}

// ─────────────────────────────────────────────────────────────────────────
// Section 4 — band evidence (the Issue-089 feed)
// ─────────────────────────────────────────────────────────────────────────

/// Fold admitted `HeldOut` rows into the frontier-report feed: the solving
/// rule gains `f=1` (the pre-solution failure) and `r=1` (the post-solution
/// success) per row — p̂=0.5, `BandClass::InBand`, the CONSTRUCTION feed
/// (ε-priced by `frontier_pairs::provenance_weight`; never rating evidence —
/// the 089-T5 fence).
#[must_use]
pub fn band_rows(admitted: &[AdmittedRow]) -> Vec<(String, u32, u32)> {
    // (rule, r, f) — summarize's (rule, successes, fails) order.
    let mut per_rule: Vec<(String, u32, u32)> = Vec::new();
    for row in admitted.iter().filter(|r| r.mode == Mode::HeldOut) {
        let Some(rule) = &row.solving_rule else { continue };
        match per_rule.iter_mut().find(|(r, _, _)| r == rule) {
            Some((_, r, f)) => {
                *r += 1;
                *f += 1;
            }
            None => per_rule.push((rule.clone(), 1, 1)),
        }
    }
    per_rule.shrink_to_fit();
    per_rule
}

/// The band summary over admitted rows (substrate: `frontier_report`).
#[must_use]
pub fn band_summary(admitted: &[AdmittedRow]) -> frontier_report::FrontierSummary {
    frontier_report::summarize(&band_rows(admitted))
}

// ─────────────────────────────────────────────────────────────────────────
// Section 5 — the row store (append-only, BLAKE3-deduped)
// ─────────────────────────────────────────────────────────────────────────

/// One committed jsonl row (serde shape — `ts`/`sha` are commit-time facts,
/// deliberately OUTSIDE the derived-set hash so G4 stays machine-stable).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct MinerRow {
    pub ts: String,
    pub sha: String,
    pub family: String,
    pub target_lint: String,
    pub ops: u32,
    pub blake3: String,
    pub mode: String,
    pub solving_rule: Option<String>,
    pub lints_before: Vec<String>,
    pub lints_after: Vec<String>,
    pub seed_blake3: String,
    pub provenance: String,
}

/// The exactly-once memory loaded from the row store: content addresses
/// AND the families that already hold a committed row — K-minimality holds
/// ACROSS passes (a family with a committed row never admits its next
/// candidate while that row stands; a mining batch that lands the rule
/// re-opens the family by archiving the row).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KnownSet {
    /// Committed fixture content addresses (BLAKE3).
    pub blake3s: BTreeSet<String>,
    /// Families with a committed row this epoch.
    pub families: BTreeSet<String>,
}

impl KnownSet {
    /// The empty set (a first pass).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
/// Load the committed exactly-once memory (content addresses + committed
/// families) from the row store.
#[must_use]
pub fn load_known(path: &std::path::Path) -> KnownSet {
    let mut known = KnownSet::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return known;
    };
    for row in text.lines().filter_map(|l| serde_json::from_str::<MinerRow>(l).ok()) {
        known.blake3s.insert(row.blake3);
        known.families.insert(row.family);
    }
    known
}

/// Append committed rows (best-effort, append-only — the caller owns the
/// commit boundary).
///
/// # Errors
/// Propagates I/O + serde failures.
pub fn append_rows(path: &std::path::Path, rows: &[AdmittedRow]) -> std::io::Result<()> {
    use std::io::Write as _;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    for row in rows {
        let m = MinerRow {
            ts: frontier_report::local_iso_timestamp(),
            sha: frontier_report::git_sha(),
            family: row.family.to_string(),
            target_lint: row.target_lint.to_string(),
            ops: row.ops,
            blake3: row.blake3.clone(),
            mode: match row.mode {
                Mode::HeldOut => "heldout",
                Mode::Frontier => "frontier",
            }.to_string(),
            solving_rule: row.solving_rule.clone(),
            lints_before: row.lints_before.clone(),
            lints_after: row.lints_after.clone(),
            seed_blake3: row.seed_blake3.clone(),
            provenance: PROVENANCE.to_string(),
        };
        let line = serde_json::to_string(&m).map_err(std::io::Error::other)?;
        writeln!(f, "{line}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::clippy_oracle::OracleVerdict;
    use crate::frontier_report::BandClass;

    fn verdict(ran: bool, compiles: bool, lints: &[&str]) -> OracleVerdict {
        OracleVerdict {
            ran,
            compiles,
            lints: lints.iter().map(|s| (*s).to_string()).collect(),
            error: None,
        }
    }

    /// The HeldOut-mode probe: seeds and proposer-healed forms are clean;
    /// anything still carrying `return ` fires `needless_return` (the
    /// view-healed output is byte-identical to the broken form — that is
    /// the unsolved-before fact under test).
    fn heldout_probe(code: &str) -> OracleVerdict {
        if code.contains("return ") {
            verdict(true, true, &["needless_return"])
        } else {
            verdict(true, true, &[])
        }
    }

    /// The Frontier-mode probe: fires the family's target lint on any text
    /// carrying the family's broken-shape marker (seeds never match; the
    /// default-view heal leaves the text untouched — nothing fires — so the
    /// healed verdict equals the broken verdict, which is the frontier
    /// fact under test).
    fn frontier_probe(code: &str) -> OracleVerdict {
        // redundant_slicing BEFORE needless_borrow: `first_char(&s[..])`
        // matches a bare `(&` too — the marker order is load-bearing.
        let lint = if code.contains("&s[..]") {
            "redundant_slicing"
        } else if code.contains("let out =") || code.contains("let sum =") || code.contains("let made =") {
            "let_and_return"
        } else if code.contains("&Vec<") {
            "ptr_arg"
        } else if code.contains("(&s)") || code.contains("(&v)") || code.contains("(&s.trim())") {
            "needless_borrow"
        } else if code.contains(").flatten()") {
            "map_flatten"
        } else {
            return verdict(true, true, &[]);
        };
        verdict(true, true, &[lint])
    }

    #[test]
    fn generate_is_deterministic_and_canonical() {
        let a = generate(&[Mode::HeldOut, Mode::Frontier]);
        let b = generate(&[Mode::HeldOut, Mode::Frontier]);
        assert_eq!(a, b, "same inputs must give byte-identical candidates");
        assert_eq!(derived_set_blake3(&a), derived_set_blake3(&b));
        // Canonical order: family asc, blake3 asc — asserted, not assumed.
        let mut sorted = a.clone();
        sorted.sort_by(|x, y| x.family.cmp(y.family).then_with(|| x.blake3.cmp(&y.blake3)));
        assert_eq!(a, sorted);
        // Every candidate is 1-op (the table invariant; ops≥2 is future work).
        assert!(a.iter().all(|c| c.ops == 1));
        // HeldOut + Frontier partition by the family's covered flag.
        assert_eq!(a.iter().filter(|c| c.mode == Mode::HeldOut).count(), 3);
        assert_eq!(a.iter().filter(|c| c.mode == Mode::Frontier).count(), 13);
    }

    #[test]
    fn derived_set_blake3_is_sixteen_hex() {
        let set = generate(&[Mode::Frontier]);
        let h = derived_set_blake3(&set);
        assert_eq!(h.len(), 16);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn heldout_pass_admits_one_row_per_family_with_full_invariant() {
        let cands = generate(&[Mode::HeldOut]);
        let sol = Solution::Reenable(HELD_OUT_RULE.to_string());
        let out = admit(&cands, &CorpusView::held_out(), Some(&sol), &KnownSet::new(), &heldout_probe);
        assert_eq!(out.admitted.len(), 1, "K-minimality: first admissible per family");
        let row = &out.admitted[0];
        assert_eq!(row.family, "needless_return-heldout");
        assert_eq!(row.mode, Mode::HeldOut);
        assert_eq!(row.solving_rule.as_deref(), Some(HELD_OUT_RULE));
        assert!(out.rejections.is_empty(), "later family members are skipped, not rejected");
        // The committed row is the canonical-first (lowest blake3 at ops=1).
        let first = cands
            .iter()
            .min_by(|x, y| x.blake3.cmp(&y.blake3))
            .expect("non-empty family");
        assert_eq!(row.blake3, first.blake3);
    }

    #[test]
    fn unsolved_before_is_real_the_view_leaves_the_lint_firing() {
        // Ground truth for the mock: the view-healed output IS the broken
        // text (the only rule that could touch it is held out) — assert the
        // pipeline fact the mock probe stands in for.
        let cands = generate(&[Mode::HeldOut]);
        let (healed, edits, _) = heal_fixpoint_cfg(
            &cands[0].broken,
            &[],
            &CorpusView::held_out().override_for(),
        );
        assert_eq!(healed, cands[0].broken, "view heal must leave the fixture untouched");
        assert_eq!(edits, 0);
        // And the default view (the solution) DOES clear it.
        let (solved, edits, _) = heal_fixpoint_cfg(
            &cands[0].broken,
            &[],
            &ComponentOverride::default(),
        );
        assert_ne!(solved, cands[0].broken);
        assert!(edits > 0);
        assert!(!solved.contains("return "));
    }

    #[test]
    fn duplicate_blake3_is_rejected_exactly_once_holds() {
        let cands = generate(&[Mode::HeldOut]);
        let sol = Solution::Reenable(HELD_OUT_RULE.to_string());
        let first = admit(&cands, &CorpusView::held_out(), Some(&sol), &KnownSet::new(), &heldout_probe);
        assert_eq!(first.admitted.len(), 1);
        // Cross-pass K-minimality: the family with a committed row is DONE —
        // no silent second admission from the same family.
        let known = KnownSet {
            blake3s: first.admitted.iter().map(|r| r.blake3.clone()).collect(),
            families: first.admitted.iter().map(|r| r.family.to_string()).collect(),
        };
        let second = admit(&cands, &CorpusView::held_out(), Some(&sol), &known, &heldout_probe);
        assert!(second.admitted.is_empty());
        assert!(second.rejections.is_empty(), "family-done is a skip, not a rejection");
        // A known CONTENT ADDRESS in an uncommitted family is the exactly-once
        // rejection (the cross-family collision path).
        let mut orphan = KnownSet::new();
        orphan.blake3s.insert(first.admitted[0].blake3.clone());
        let third = admit(&cands, &CorpusView::held_out(), Some(&sol), &orphan, &heldout_probe);
        assert_eq!(third.admitted.len(), 1, "the family's NEXT candidate admits");
        assert!(third
            .rejections
            .iter()
            .any(|r| matches!(r, Rejection::Duplicate { .. })));
    }

    #[test]
    fn broad_entry_solution_that_perturbs_a_stock_row_is_refused() {
        // A solution that CLEARS the heldout fixture but perturbs the stock
        // needless_return row's heal output (byte-equality discipline: even
        // a whitespace delta is a perturbation) must REFUSE — the
        // adversarial G1 arm.
        let cands = generate(&[Mode::HeldOut]);
        let broad = Solution::Entry {
            buggy: "return ".to_string(),
            fixed: String::new(),
        };
        let out = admit(&cands, &CorpusView::held_out(), Some(&broad), &KnownSet::new(), &heldout_probe);
        assert!(out.admitted.is_empty(), "the adversarial solution must not commit");
        assert!(out
            .rejections
            .iter()
            .any(|r| matches!(r, Rejection::RegressionRefused { .. })));
        // Directly: the perturbation names the stock row it re-broke.
        let perturbed = regression_perturbations(&broad).expect("must refuse");
        assert!(perturbed.contains(&"needless_return".to_string()));
    }

    #[test]
    fn narrow_entry_solution_admits() {
        // The surgical entry: clears the fixture, touches no stock row.
        let cands = generate(&[Mode::HeldOut]);
        let narrow = Solution::Entry {
            buggy: "return x * 2;".to_string(),
            fixed: "x * 2".to_string(),
        };
        assert_eq!(regression_perturbations(&narrow), None);
        let out = admit(&cands, &CorpusView::held_out(), Some(&narrow), &KnownSet::new(), &heldout_probe);
        assert_eq!(out.admitted.len(), 1);
    }

    #[test]
    fn dirty_seed_rejects_loudly() {
        let dirty = |_: &str| verdict(true, true, &["let_and_return"]);
        let cands = generate(&[Mode::HeldOut]);
        let sol = Solution::Reenable(HELD_OUT_RULE.to_string());
        let out = admit(&cands, &CorpusView::held_out(), Some(&sol), &KnownSet::new(), &dirty);
        assert!(out.admitted.is_empty());
        assert!(out
            .rejections
            .iter()
            .all(|r| matches!(r, Rejection::SeedNotClean { .. })));
    }

    #[test]
    fn target_lint_not_fired_rejects_with_what_was_seen() {
        // Clean on seeds, WRONG lint on the broken form (family drift).
        let blind = |code: &str| {
            if code.contains("return ") {
                verdict(true, true, &["something_else"])
            } else {
                verdict(true, true, &[])
            }
        };
        let cands = generate(&[Mode::HeldOut]);
        let sol = Solution::Reenable(HELD_OUT_RULE.to_string());
        let out = admit(&cands, &CorpusView::held_out(), Some(&sol), &KnownSet::new(), &blind);
        assert!(out.admitted.is_empty());
        assert!(out
            .rejections
            .iter()
            .all(|r| matches!(r, Rejection::TargetLintNotFired { .. })));
    }

    #[test]
    fn frontier_rows_are_pending_leads_and_default_view_cannot_heal() {
        // Pipeline fact first: the default view leaves every frontier
        // broken form untouched (no rule in the compiled set fires).
        let cands = generate(&[Mode::Frontier]);
        for c in &cands {
            let (healed, edits, _) = heal_fixpoint_cfg(&c.broken, &[], &ComponentOverride::default());
            assert_eq!(healed, c.broken, "family {} must be unhealable today", c.family);
            assert_eq!(edits, 0);
        }
        let out = admit(&cands, &CorpusView::default(), None, &KnownSet::new(), &frontier_probe);
        assert_eq!(out.admitted.len(), 5, "one per frontier family (K-minimality)");
        assert!(out
            .admitted
            .iter()
            .all(|r| r.mode == Mode::Frontier && r.solving_rule.is_none()));
        // Band fold: frontier rows are NOT band evidence (no solving rule).
        assert!(band_rows(&out.admitted).is_empty());
        let summary = band_summary(&out.admitted);
        assert_eq!(summary.population, 0, "green-zero shape for frontier-only passes");
    }

    #[test]
    fn band_rows_land_in_band_via_the_frontier_report_substrate() {
        let cands = generate(&[Mode::HeldOut]);
        let sol = Solution::Reenable(HELD_OUT_RULE.to_string());
        let out = admit(&cands, &CorpusView::held_out(), Some(&sol), &KnownSet::new(), &heldout_probe);
        let rows = band_rows(&out.admitted);
        assert_eq!(rows, vec![(HELD_OUT_RULE.to_string(), 1, 1)]);
        let summary = frontier_report::summarize(&rows);
        assert_eq!(summary.seen, 1);
        assert_eq!(summary.band_count, 1);
        assert!((summary.band_mass - 1.0).abs() < 1e-6);
        assert_eq!(summary.rows[0].class, BandClass::InBand, "p̂=0.5 is the band edge");
        // Provenance discipline: the feed is construction-priced (ε), never
        // organic — pin the vocabulary.
        assert_eq!(rows.len(), 1);
        let eps = crate::frontier_pairs::SYNTHETIC_PAIR_WEIGHT
            - crate::frontier_pairs::provenance_weight(PROVENANCE);
        assert!(eps.abs() < 1e-7);
    }

    #[test]
    fn jsonl_round_trips_and_dedups() {
        let dir = std::env::temp_dir().join("riir_frontier_miner_tests");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("rows.jsonl");
        let cands = generate(&[Mode::HeldOut]);
        let sol = Solution::Reenable(HELD_OUT_RULE.to_string());
        let out = admit(&cands, &CorpusView::held_out(), Some(&sol), &KnownSet::new(), &heldout_probe);
        append_rows(&path, &out.admitted).expect("append");
        let known = load_known(&path);
        assert_eq!(known.blake3s.len(), 1);
        assert!(known.blake3s.contains(&out.admitted[0].blake3));
        assert!(known.families.contains("needless_return-heldout"));
        // Exactly-once holds across a reload.
        let second = admit(&cands, &CorpusView::held_out(), Some(&sol), &known, &heldout_probe);
        assert!(second.admitted.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn seed_corpus_commitment_is_untouched() {
        // G3: the miner is read-only over the committed corpus — same
        // entries, same BLAKE3 commitment, before and after any miner use.
        let a = crate::build_seed_corpus();
        let cands = generate(&[Mode::HeldOut, Mode::Frontier]);
        assert!(!cands.is_empty());
        let b = crate::build_seed_corpus();
        assert_eq!(a.commitment, b.commitment);
        assert_eq!(a.entries.len(), b.entries.len());
        assert!(a.verify());
    }
}

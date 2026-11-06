//! L3 — Ruliology Pareto-front candidate search.
//!
//! When the L1 drafter proposes multiple candidates and L2 accepts more than
//! one, this module enumerates the Pareto-front of (`lint_satisfied`,
//! AST delta size) and picks the minimum-complexity fix among the
//! Pareto-optimal set.
//!
//! # Evidence axis (Issue 039 T3)
//!
//! `ScoredCandidate` carries a third axis, `evidence: f32` — the rule's ELO
//! rating from the trajectory store ([`crate::elo`], base 1200 when unrated).
//! It is a **lexicographic tie-break ONLY**: payoff stays primary, and the
//! sort is
//!
//! ```text
//! (lint_satisfied desc, ast_delta_size asc, pareto_rank asc, evidence desc)
//! ```
//!
//! so evidence reorders exactly the candidates that tie on payoff +
//! complexity + rank — where the pre-039 sort fell to arbitrary input order.
//! Measured track record decides what structure alone cannot. Evidence NEVER
//! participates in Pareto dominance (a rated candidate does not dominate an
//! unrated one — [`dominates`] is unchanged) and NEVER outranks payoff: a
//! parse-failing high-ELO candidate stays below every parse-passing one
//! (correctness before rating — pinned by test).
//!
//! # Substrate lineage
//!
//! The Pareto-front algorithm is reimplemented locally from
//! `katgpt-rs/crates/katgpt-ruliology/src/types.rs::WinMatrix::pareto_front`
//! (GOAT PASS, Bench 572). The substrate's version is coupled to `WinMatrix`
//! (2-player tournament semantics) and non-optionally pulls `katgpt-pruners`
//! (`g_zero` feature) — too heavy for a ~30 LOC algorithm. Clippy fixes have
//! ground-truth payoff (`lint_satisfied`: bool from cargo clippy), not pairwise
//! tournament payoffs, so the `WinMatrix` type doesn't fit.
//!
//! This mirrors the repo's existing pattern: `ClippyTernaryDrafter` reuses the
//! `TernaryDraftModel` *pattern* from `quest_grammar` without a code-level dep
//! (riir-ai AGENTS.md "pattern borrowing ≠ dependency" rule).
//!
//! # Modelless
//!
//! Entirely modelless: pure AST diff + Pareto dominance check. No training,
//! no backprop. Runs once per lint invocation (not per-tick).

use crate::domain::HealEntry;
use crate::elo::EloTable;

/// A candidate fix scored for Pareto-front ranking.
///
/// `lint_satisfied` is the payoff (higher is better); `ast_delta_size` is the
/// complexity (lower is better). A candidate dominates another if it has
/// payoff ≥ AND strictly lower complexity (with at least one strict inequality).
///
/// `evidence` (Issue 039 T3) is the rule's ELO rating from the trajectory
/// store — a lexicographic tie-break ONLY (see the module doc): it reorders
/// candidates that tie on payoff + complexity + rank, nothing else.
///
/// Generic over `E: HealEntry` (Plan 001 Phase 1) — works with any domain's
/// corpus entry type.
#[derive(Debug, Clone)]
pub struct ScoredCandidate<'a, E: HealEntry> {
    /// The corpus entry being scored.
    pub entry: &'a E,
    /// Payoff: true if L2 accepts (parses) the fixed code. When the
    /// `clippy_verify` feature is also enabled, callers can substitute the
    /// stricter `lint_resolved` signal from `validate_with_clippy`.
    pub lint_satisfied: bool,
    /// Complexity: count of changed AST nodes between `buggy_code` and
    /// `fixed_code`. Lower = simpler fix = preferred.
    pub ast_delta_size: usize,
    /// Rank in the Pareto-front: 0 = Pareto-optimal, higher = dominated by
    /// more candidates. Candidates with the same `pareto_rank` are
    /// incomparable (neither dominates the other).
    pub pareto_rank: usize,
    /// Evidence (Issue 039 T3): the rule's ELO rating over the trajectory
    /// store ([`crate::elo`]), f32 to match it; [`crate::elo::default_elo`]
    /// (1200) when the rule has no rated history. Lexicographic tie-break
    /// ONLY — excluded from Pareto dominance, never outranks payoff.
    pub evidence: f32,
}

/// The L3 ruliology search engine.
///
/// Stateless — holds no scratch buffers because the candidate counts per lint
/// are small (typically <10). The AST walk allocates a `Vec` for node
/// traversal, but this is per-search (not per-tick), so the alloc-free
/// steady-state gate (G4) doesn't apply.
#[derive(Debug, Clone)]
pub struct RuliologySearch {
    /// The L2 pruner used to compute `lint_satisfied` for each candidate.
    /// Owned so the search is self-contained.
    pruner: crate::pruner::RustAnalyzerPruner,
}

impl Default for RuliologySearch {
    fn default() -> Self {
        Self::new()
    }
}

impl RuliologySearch {
    /// Create a new search engine with the default L2 pruner.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pruner: crate::pruner::RustAnalyzerPruner::new(),
        }
    }

    /// Create with a specific pruner (for testing or composition).
    #[must_use]
    pub fn with_pruner(pruner: crate::pruner::RustAnalyzerPruner) -> Self {
        Self { pruner }
    }

    /// Score and rank all candidates for a lint.
    ///
    /// Each candidate is scored:
    /// - `lint_satisfied`: true if the fixed code parses (L2 fast path).
    ///   When `clippy_verify` is also enabled, callers should re-score with
    ///   `validate_with_clippy` for truthful lint resolution.
    /// - `ast_delta_size`: count of changed syn nodes between buggy + fixed.
    /// - `pareto_rank`: 0 = Pareto-optimal.
    /// - `evidence`: base 1200 for every candidate (unrated) — the tie-break
    ///   is a no-op and the ordering is identical to the pre-039 search.
    ///
    /// Returns all candidates (not just the Pareto-front), sorted by
    /// `(lint_satisfied desc, ast_delta_size asc, pareto_rank asc, evidence desc)`.
    /// Callers typically take `result[0]` or filter `pareto_rank == 0`.
    ///
    /// Generic over `E: HealEntry` (Plan 001 Phase 1) — works with any domain's
    /// corpus entry type.
    #[must_use]
    pub fn search<'c, E: HealEntry>(
        &self,
        buggy_code: &str,
        candidates: &'c [E],
    ) -> Vec<ScoredCandidate<'c, E>> {
        // Unrated search: an empty table leaves every candidate at base
        // evidence (zero-alloc HashMap) — bit-identical to the pre-039 order.
        self.search_with_evidence(buggy_code, candidates, &EloTable::new())
    }

    /// Score and rank candidates with real-world evidence (Issue 039 T3).
    ///
    /// Same scoring + ordering as [`search`](Self::search), but each
    /// candidate's `evidence` is its rule's ELO rating looked up in `ratings`
    /// (`ratings.rating(entry.rule_id())`; base 1200 for rules with no rated
    /// history). Evidence is a lexicographic tie-break ONLY — see the module
    /// doc — so this changes the order of exactly the candidates that tie on
    /// payoff + complexity + rank.
    ///
    /// The natural caller bridge (the store keeps final ratings denormalized
    /// on every row, the 039-T2 consumer contract): build the table from
    /// `TrajStore` rows after `recompute_elo`, then pass it here. The search
    /// itself never touches the store — batch-on-load only (the issue's
    /// non-goal: no ELO work inside the hot fix loop).
    #[must_use]
    pub fn search_with_evidence<'c, E: HealEntry>(
        &self,
        buggy_code: &str,
        candidates: &'c [E],
        ratings: &EloTable,
    ) -> Vec<ScoredCandidate<'c, E>> {
        if candidates.is_empty() {
            return Vec::new();
        }

        // Phase 1: score each candidate.
        let mut scored: Vec<ScoredCandidate<'c, E>> = candidates
            .iter()
            .map(|entry| {
                let result = self.pruner.validate(entry.fixed_code(), entry.rule_id());
                let ast_delta = ast_delta_size(buggy_code, entry.fixed_code());
                ScoredCandidate {
                    entry,
                    lint_satisfied: result.parses,
                    ast_delta_size: ast_delta,
                    pareto_rank: 0, // filled in Phase 2
                    evidence: ratings.rating(entry.rule_id()),
                }
            })
            .collect();

        // Phase 2: compute Pareto rank for each candidate.
        // Rank = number of other candidates that dominate this one.
        // Pareto-optimal = rank 0. Evidence is deliberately EXCLUDED from
        // dominance (tie-break only — the front shape is unchanged).
        let n = scored.len();
        for i in 0..n {
            let dominated_by = (0..n)
                .filter(|&j| j != i && dominates(&scored[j], &scored[i]))
                .count();
            scored[i].pareto_rank = dominated_by;
        }

        // Phase 3: sort by
        // (lint_satisfied desc, ast_delta_size asc, pareto_rank asc,
        //  evidence desc).
        // This puts the best candidate first: satisfying + simplest +
        // Pareto-optimal + (Issue 039 T3) higher-rated among full ties.
        scored.sort_by(cmp_rank);

        scored
    }

    /// Pick the best fix: minimum-AST-delta among Pareto-optimal + satisfying candidates.
    ///
    /// Returns `None` if no candidates satisfy the L2 pruner.
    #[must_use]
    pub fn best_fix<'c, E: HealEntry>(
        &self,
        buggy_code: &str,
        candidates: &'c [E],
    ) -> Option<&'c E> {
        self.best_fix_with_evidence(buggy_code, candidates, &EloTable::new())
    }

    /// [`best_fix`](Self::best_fix) with real-world evidence (Issue 039 T3):
    /// among equally-satisfying, equal-complexity, Pareto-optimal candidates
    /// the higher-rated rule wins. Payoff still dominates — a parse-passing
    /// candidate is never displaced by a higher-rated parse-failing one.
    #[must_use]
    pub fn best_fix_with_evidence<'c, E: HealEntry>(
        &self,
        buggy_code: &str,
        candidates: &'c [E],
        ratings: &EloTable,
    ) -> Option<&'c E> {
        let scored = self.search_with_evidence(buggy_code, candidates, ratings);
        scored
            .into_iter()
            .find(|s| s.lint_satisfied && s.pareto_rank == 0)
            .map(|s| s.entry)
    }

    /// Return only the Pareto-optimal candidates (rank 0).
    #[must_use]
    pub fn pareto_optimal<'c, E: HealEntry>(
        &self,
        buggy_code: &str,
        candidates: &'c [E],
    ) -> Vec<ScoredCandidate<'c, E>> {
        self.pareto_optimal_with_evidence(buggy_code, candidates, &EloTable::new())
    }

    /// [`pareto_optimal`](Self::pareto_optimal) with real-world evidence
    /// (Issue 039 T3): the front SET is unchanged (evidence is excluded from
    /// dominance) but its presentation order breaks full ties by `evidence`
    /// desc instead of input order.
    #[must_use]
    pub fn pareto_optimal_with_evidence<'c, E: HealEntry>(
        &self,
        buggy_code: &str,
        candidates: &'c [E],
        ratings: &EloTable,
    ) -> Vec<ScoredCandidate<'c, E>> {
        self.search_with_evidence(buggy_code, candidates, ratings)
            .into_iter()
            .filter(|s| s.pareto_rank == 0)
            .collect()
    }
}

/// Does candidate `a` dominate candidate `b`?
///
/// `a` dominates `b` iff:
/// - `a.lint_satisfied >= b.lint_satisfied` (payoff ≥), AND
/// - `a.ast_delta_size <= b.ast_delta_size` (complexity ≤), AND
/// - at least one of those is strict (≠).
///
/// Direct adaptation of `WinMatrix::pareto_front`'s dominance check
/// (`katgpt-ruliology/src/types.rs` lines 119-126), with bool payoff +
/// usize complexity in place of f64/f32.
///
/// NOTE (Issue 039 T3): `evidence` is deliberately EXCLUDED — it is a
/// lexicographic tie-break in [`cmp_rank`], not a front axis. A rated
/// candidate does not dominate an unrated one; ratings reorder ties only.
#[inline]
fn dominates<E: HealEntry>(a: &ScoredCandidate<'_, E>, b: &ScoredCandidate<'_, E>) -> bool {
    let payoff_ge = a.lint_satisfied >= b.lint_satisfied; // bool: true >= false
    let complexity_le = a.ast_delta_size <= b.ast_delta_size;
    let strictly_better =
        a.lint_satisfied != b.lint_satisfied || a.ast_delta_size != b.ast_delta_size;
    payoff_ge && complexity_le && strictly_better
}

/// The search's total order (Issue 039 T3 makes it explicit + tested):
///
/// ```text
/// (lint_satisfied desc, ast_delta_size asc, pareto_rank asc, evidence desc)
/// ```
///
/// Payoff is primary — a parse-passing candidate sorts before every
/// parse-failing one regardless of ELO. Complexity still beats evidence
/// among parse-equal candidates (the front stays `(payoff, complexity)`;
/// evidence is the third, tie-only axis). Evidence (`f32` ELO, finite)
/// compares via `total_cmp` for a deterministic total order.
#[inline]
fn cmp_rank<E: HealEntry>(
    a: &ScoredCandidate<'_, E>,
    b: &ScoredCandidate<'_, E>,
) -> std::cmp::Ordering {
    // lint_satisfied: true > false
    match (a.lint_satisfied, b.lint_satisfied) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => {
            // ast_delta_size: lower is better
            a.ast_delta_size
                .cmp(&b.ast_delta_size)
                .then_with(|| a.pareto_rank.cmp(&b.pareto_rank))
                // evidence: higher is better (Issue 039 T3 tie-break)
                .then_with(|| b.evidence.total_cmp(&a.evidence))
        }
    }
}

/// Count the AST delta size between buggy and fixed code.
///
/// This is the complexity metric for the Pareto-front. We count the total
/// number of AST nodes in `fixed_code` that are NOT present (by position +
/// kind) in `buggy_code`. Lower = simpler fix.
///
/// Uses a structural walk: parse both as `syn::File`, then count nodes via
/// `Visit` traversal. Two nodes "match" if they have the same kind at the
/// same position in the pre-order traversal. This is a simple O(n+m) walk,
/// not a tree-edit-distance (which would be O(n·m) — deferred).
///
/// If either code fails to parse, returns `usize::MAX` (the fix is invalid —
/// worst possible complexity).
#[must_use]
pub fn ast_delta_size(buggy_code: &str, fixed_code: &str) -> usize {
    let Some(buggy_nodes) = collect_ast_nodes(buggy_code) else {
        return usize::MAX; // buggy code unparseable
    };
    let Some(fixed_nodes) = collect_ast_nodes(fixed_code) else {
        return usize::MAX; // fixed code unparseable
    };

    // Count fixed nodes that don't appear at the same position in buggy.
    // This is a position-sensitive count: if fixed has a node at index i with
    // kind K, and buggy's i-th node is also K, it's "shared"; otherwise it's
    // "changed". Lower count = fewer changes = simpler fix.
    let mut changed = 0usize;
    for (i, fixed_kind) in fixed_nodes.iter().enumerate() {
        match buggy_nodes.get(i) {
            Some(buggy_kind) if buggy_kind == fixed_kind => { /* shared — no change */ }
            _ => changed += 1,
        }
    }
    // Also count nodes that exist in buggy but not fixed (deletions count).
    if fixed_nodes.len() < buggy_nodes.len() {
        changed += buggy_nodes.len() - fixed_nodes.len();
    }
    changed
}

/// Collect the pre-order traversal of AST node kinds for a code snippet.
///
/// Returns `None` if the code doesn't parse as a `syn::File`. Each entry is
/// a stringified kind (e.g., `"Item::Fn"`, `"Expr::Lit"`, `"Stmt::Local"`).
fn collect_ast_nodes(code: &str) -> Option<Vec<String>> {
    let file = syn::parse_file(code).ok()?;
    let mut collector = AstNodeCollector::default();
    syn::visit::visit_file(&mut collector, &file);
    Some(collector.nodes)
}

/// A `syn::visit` implementation that collects the kind of each visited node.
///
/// We only record the *kind* (not the content) — this gives a structural
/// signature suitable for delta-size computation. Two fixes with the same
/// structural shape (e.g., both rewrite `return X;` → `X`) get the same
/// delta size, which is correct for Pareto ranking.
#[derive(Default)]
struct AstNodeCollector {
    nodes: Vec<String>,
}

// ── syn::visit impl ─────────────────────────────────────────────────────
//
// We override the per-node-kind visit methods to push the kind name. We only
// override the common node types — `syn::visit` has hundreds. For unhandled
// nodes, the default visitor walks children without recording, which is fine
// (we get a partial signature; the delta-size is still a valid relative
// complexity proxy).
//
// The node kinds recorded mirror the substrate's approach: each AST node
// contributes one entry. The exact kind taxonomy doesn't matter for delta
// computation — what matters is that DIFFERENT structures produce DIFFERENT
// signatures, so the delta size reflects real structural change.

impl<'ast> syn::visit::Visit<'ast> for AstNodeCollector {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        self.nodes.push(format!("ItemFn:{}", i.sig.ident));
        syn::visit::visit_item_fn(self, i);
    }

    fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
        self.nodes.push(format!("ItemStruct:{}", i.ident));
        syn::visit::visit_item_struct(self, i);
    }

    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        self.nodes.push("ItemImpl".to_string());
        syn::visit::visit_item_impl(self, i);
    }

    fn visit_stmt(&mut self, i: &'ast syn::Stmt) {
        let kind = match i {
            syn::Stmt::Local(_) => "Stmt::Local",
            syn::Stmt::Item(_) => "Stmt::Item",
            syn::Stmt::Expr(_, _) => "Stmt::Expr",
            syn::Stmt::Macro(_) => "Stmt::Macro",
        };
        self.nodes.push(kind.to_string());
        syn::visit::visit_stmt(self, i);
    }

    fn visit_expr(&mut self, i: &'ast syn::Expr) {
        let kind = match i {
            syn::Expr::Lit(_) => "Expr::Lit",
            syn::Expr::Binary(_) => "Expr::Binary",
            syn::Expr::Call(_) => "Expr::Call",
            syn::Expr::MethodCall(_) => "Expr::MethodCall",
            syn::Expr::Path(_) => "Expr::Path",
            syn::Expr::If(_) => "Expr::If",
            syn::Expr::Match(_) => "Expr::Match",
            syn::Expr::Block(_) => "Expr::Block",
            syn::Expr::Return(_) => "Expr::Return",
            syn::Expr::Closure(_) => "Expr::Closure",
            syn::Expr::Field(_) => "Expr::Field",
            syn::Expr::Index(_) => "Expr::Index",
            syn::Expr::Unary(_) => "Expr::Unary",
            syn::Expr::Assign(_) => "Expr::Assign",
            _ => "Expr::Other",
        };
        self.nodes.push(kind.to_string());
        syn::visit::visit_expr(self, i);
    }

    fn visit_local(&mut self, i: &'ast syn::Local) {
        self.nodes.push("Local".to_string());
        syn::visit::visit_local(self, i);
    }

    fn visit_pat(&mut self, i: &'ast syn::Pat) {
        let kind = match i {
            syn::Pat::Ident(_) => "Pat::Ident",
            syn::Pat::Tuple(_) => "Pat::Tuple",
            syn::Pat::TupleStruct(_) => "Pat::TupleStruct",
            syn::Pat::Wild(_) => "Pat::Wild",
            _ => "Pat::Other",
        };
        self.nodes.push(kind.to_string());
        syn::visit::visit_pat(self, i);
    }

    fn visit_field_value(&mut self, i: &'ast syn::FieldValue) {
        // syn 2.x: FieldValue has `member`, not `ident`.
        let member_str = match &i.member {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(idx) => idx.index.to_string(),
        };
        self.nodes.push(format!("FieldValue:{member_str}"));
        syn::visit::visit_field_value(self, i);
    }
}

// ── Pareto-front algorithm (reimplemented from substrate) ───────────────
//
// Public for testing — allows direct unit testing of the dominance check
// without constructing full `ClippyFixEntry` + `RustAnalyzerPruner` fixtures.

/// Compute the Pareto-front: indices of candidates that are NOT dominated.
///
/// Direct adaptation of `WinMatrix::pareto_front` from
/// `katgpt-rs/crates/katgpt-ruliology/src/types.rs` lines 74-131.
/// O(n²) dominance check — acceptable for small per-lint candidate counts
/// (typically <10).
///
/// A candidate `i` is dominated iff some `j != i` has:
/// - `payoffs[j] >= payoffs[i]` (payoff ≥), AND
/// - `complexities[j] < complexities[i]` (strictly lower complexity).
///
/// Note: this is the substrate's strict definition. The `dominates` helper
/// above uses the non-strict version (≤) for Pareto *rank* computation.
#[must_use]
pub fn pareto_front(payoffs: &[bool], complexities: &[usize]) -> Vec<usize> {
    let n = payoffs.len();
    debug_assert_eq!(
        complexities.len(),
        n,
        "payoffs and complexities must have equal length"
    );
    if n == 0 {
        return Vec::new();
    }

    let mut front = Vec::with_capacity(n);
    for i in 0..n {
        // Dominated iff some j has payoff ≥ AND strictly lower complexity.
        let dominated =
            (0..n).any(|j| j != i && payoffs[j] >= payoffs[i] && complexities[j] < complexities[i]);
        if !dominated {
            front.push(i);
        }
    }
    front
}

#[cfg(all(test, feature = "clippy_lints"))]
mod tests {
    use super::*;
    use crate::domains::clippy_lints::corpus::build_seed_corpus;
    use crate::domains::clippy_lints::ClippyFixEntry;
    use crate::elo::default_elo;

    fn mock_entry(lint: &str, buggy: &str, fixed: &str) -> ClippyFixEntry {
        ClippyFixEntry {
            lint_name: lint.to_string(),
            lint_group: "style".to_string(),
            buggy_code: buggy.to_string(),
            fixed_code: fixed.to_string(),
        }
    }

    // ── pareto_front unit tests (G1 correctness) ────────────────────────

    #[test]
    fn pareto_front_empty() {
        let front = pareto_front(&[], &[]);
        assert_eq!(front, [] as [usize; 0]);
    }

    #[test]
    fn pareto_front_single_candidate() {
        let front = pareto_front(&[true], &[5]);
        assert_eq!(front, vec![0]);
    }

    #[test]
    fn pareto_front_filters_dominated() {
        // Candidate 0: payoff=true, complexity=10
        // Candidate 1: payoff=true, complexity=5  ← dominates 0 (same payoff, lower complexity)
        // Expect: only candidate 1 on the front.
        let front = pareto_front(&[true, true], &[10, 5]);
        assert_eq!(front, vec![1]);
    }

    #[test]
    fn pareto_front_keeps_incomparable() {
        // Candidate 0: payoff=true, complexity=10
        // Candidate 1: payoff=false, complexity=5
        // Neither dominates: 0 has higher payoff, 1 has lower complexity.
        // Both are Pareto-optimal.
        let front = pareto_front(&[true, false], &[10, 5]);
        assert_eq!(front.len(), 2);
        assert!(front.contains(&0));
        assert!(front.contains(&1));
    }

    #[test]
    fn pareto_front_all_satisfy_picks_min_complexity() {
        // Three candidates, all satisfy, different complexities.
        // Front = just the minimum-complexity one.
        let front = pareto_front(&[true, true, true], &[10, 5, 7]);
        assert_eq!(front, vec![1]); // complexity 5 wins
    }

    #[test]
    fn pareto_front_mixed_satisfaction() {
        // Candidate 0: payoff=true, complexity=20  (satisfies but complex)
        // Candidate 1: payoff=false, complexity=5  (simple but doesn't satisfy)
        // Candidate 2: payoff=true, complexity=10  (satisfies + simpler than 0)
        // Candidate 3: payoff=false, complexity=3  (doesn't satisfy, simplest)
        //
        // Dominance:
        // - 2 dominates 0 (same payoff, lower complexity)
        // - 1 dominates nothing (lower payoff than 0 and 2; lower complexity only vs satisfying ones)
        // - 3 dominates 1 (same payoff=false, lower complexity)
        // Front: {2, 3} (1 dominated by 3, 0 dominated by 2)
        let front = pareto_front(&[true, false, true, false], &[20, 5, 10, 3]);
        assert!(
            front.contains(&2),
            "candidate 2 (true, 10) must be on front"
        );
        assert!(
            front.contains(&3),
            "candidate 3 (false, 3) must be on front"
        );
        assert!(
            !front.contains(&0),
            "candidate 0 (true, 20) must be dominated by 2"
        );
        assert!(
            !front.contains(&1),
            "candidate 1 (false, 5) must be dominated by 3"
        );
    }

    // ── dominates helper tests ──────────────────────────────────────────

    #[test]
    fn dominates_same_payoff_lower_complexity() {
        let a = mock_scored(true, 5);
        let b = mock_scored(true, 10);
        assert!(dominates(&a, &b));
        assert!(!dominates(&b, &a));
    }

    #[test]
    fn dominates_higher_payoff_same_complexity() {
        let a = mock_scored(true, 5);
        let b = mock_scored(false, 5);
        assert!(dominates(&a, &b));
        assert!(!dominates(&b, &a));
    }

    #[test]
    fn does_not_dominate_when_incomparable() {
        let a = mock_scored(true, 10); // higher payoff, higher complexity
        let b = mock_scored(false, 5); // lower payoff, lower complexity
        assert!(!dominates(&a, &b));
        assert!(!dominates(&b, &a));
    }

    #[test]
    fn does_not_dominate_identical() {
        let a = mock_scored(true, 5);
        let b = mock_scored(true, 5);
        assert!(!dominates(&a, &b));
        assert!(!dominates(&b, &a));
    }

    #[test]
    fn dominance_ignores_evidence() {
        // Issue 039 T3 pin: evidence is NOT a dominance axis — a rated
        // candidate does not dominate an unrated one (the front shape is
        // unchanged; ratings reorder ties in cmp_rank only).
        let rated = mock_scored_with_evidence(true, 5, 1600.0);
        let unrated = mock_scored(true, 5);
        assert!(!dominates(&rated, &unrated));
        assert!(!dominates(&unrated, &rated));
    }

    // ── Issue 039 T3: the evidence tie-break (cmp_rank) ────────────────

    #[test]
    fn cmp_payoff_dominates_evidence() {
        // Fixture (a): higher payoff wins regardless of evidence. The
        // load-bearing direction: a parse-FAILING high-ELO candidate never
        // outranks a parse-passing one — correctness before rating.
        let passing_low = mock_scored_with_evidence(true, 10, 1200.0);
        let failing_high = mock_scored_with_evidence(false, 1, 2000.0);
        assert_eq!(
            cmp_rank(&passing_low, &failing_high),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            cmp_rank(&failing_high, &passing_low),
            std::cmp::Ordering::Greater
        );
        // The trivial direction too, plus the complexity-MAX extreme (an
        // unparseable fix carries usize::MAX complexity) — payoff still first.
        let passing_high = mock_scored_with_evidence(true, usize::MAX, 1800.0);
        let failing_low = mock_scored_with_evidence(false, usize::MAX, 1200.0);
        assert_eq!(
            cmp_rank(&passing_high, &failing_low),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            cmp_rank(&failing_low, &passing_high),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn cmp_evidence_breaks_parse_equal_tie_both_directions() {
        // Fixture (b): among parse-equal (and complexity/rank-equal)
        // candidates, the higher evidence wins — in BOTH directions.
        let hi = mock_scored_with_evidence(true, 5, 1300.0);
        let lo = mock_scored_with_evidence(true, 5, 1200.0);
        assert_eq!(cmp_rank(&hi, &lo), std::cmp::Ordering::Less);
        assert_eq!(cmp_rank(&lo, &hi), std::cmp::Ordering::Greater);
        // Same tie among parse-failing candidates — evidence still breaks it
        // (ordering within the failing block is also evidence-aware).
        let hi_f = mock_scored_with_evidence(false, 5, 1300.0);
        let lo_f = mock_scored_with_evidence(false, 5, 1200.0);
        assert_eq!(cmp_rank(&hi_f, &lo_f), std::cmp::Ordering::Less);
        assert_eq!(cmp_rank(&lo_f, &hi_f), std::cmp::Ordering::Greater);
    }

    #[test]
    fn cmp_complexity_beats_evidence_among_parse_equal() {
        // The lexicographic-position pin: evidence is a tie-break ONLY —
        // complexity (the structural heuristic) still decides among
        // parse-equal candidates before evidence is ever consulted.
        let simpler_unrated = mock_scored_with_evidence(true, 3, 1200.0);
        let complex_rated = mock_scored_with_evidence(true, 5, 1600.0);
        assert_eq!(
            cmp_rank(&simpler_unrated, &complex_rated),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            cmp_rank(&complex_rated, &simpler_unrated),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn cmp_equal_evidence_is_equal_and_deterministic() {
        // Fixture (c): equal keys compare Equal (stable sort then preserves
        // input order) and the comparison is deterministic — same inputs,
        // same ordering, every time.
        let a = mock_scored_with_evidence(true, 5, 1250.0);
        let b = mock_scored_with_evidence(true, 5, 1250.0);
        assert_eq!(cmp_rank(&a, &b), std::cmp::Ordering::Equal);
        assert_eq!(cmp_rank(&b, &a), std::cmp::Ordering::Equal);
        for _ in 0..100 {
            assert_eq!(cmp_rank(&a, &b), std::cmp::Ordering::Equal);
        }
    }

    // ── Issue 039 T3: evidence-aware search (end-to-end) ──────────────

    #[test]
    fn search_payoff_beats_high_evidence() {
        // Issue 039 T3 fixture (b), end-to-end: a parse-FAILING high-ELO
        // candidate NEVER outranks a parse-passing one. The unrated rule
        // parses; the heavily-rated rule emits garbage.
        let search = RuliologySearch::new();
        let buggy = "fn foo() -> i32 { return 42; }";
        let candidates = vec![
            mock_entry("rule_a", buggy, "fn foo() -> i32 { 42 }"),
            mock_entry("rule_b", buggy, "this is not valid rust"),
        ];
        let mut ratings = EloTable::new();
        for _ in 0..4 {
            ratings.play("rule_b", "span", true); // rule_b ≈ 1258
        }
        let result = search.search_with_evidence(buggy, &candidates, &ratings);
        assert!(
            result[0].lint_satisfied,
            "parse-passing candidate must rank first regardless of ELO"
        );
        assert_eq!(result[0].entry.rule_id(), "rule_a");
        assert!(!result[1].lint_satisfied);
        assert_eq!(result[1].entry.rule_id(), "rule_b");
        // And best_fix_with_evidence picks the parse-passing one.
        let best = search.best_fix_with_evidence(buggy, &candidates, &ratings);
        assert_eq!(best.map(HealEntry::rule_id), Some("rule_a"));
    }

    #[test]
    fn search_evidence_breaks_parse_equal_tie_both_directions() {
        // End-to-end fixture (b): two candidates with IDENTICAL fixed_code
        // (equal payoff + equal complexity + equal rank 0 — the exact tie
        // that fell to input order pre-039) are reordered by rating, in
        // BOTH directions.
        let search = RuliologySearch::new();
        let buggy = "fn foo() -> i32 { return 42; }";
        let candidates = vec![
            mock_entry("rule_a", buggy, "fn foo() -> i32 { 42 }"),
            mock_entry("rule_b", buggy, "fn foo() -> i32 { 42 }"),
        ];

        // Direction 1: rule_b rated up → rule_b first.
        let mut ratings = EloTable::new();
        for _ in 0..2 {
            ratings.play("rule_b", "span", true); // ≈ 1231
        }
        let result = search.search_with_evidence(buggy, &candidates, &ratings);
        assert_eq!(result[0].entry.rule_id(), "rule_b");
        assert_eq!(result[1].entry.rule_id(), "rule_a");
        assert_eq!(result[0].pareto_rank, 0);
        assert!(result[0].evidence > result[1].evidence);

        // Direction 2: rule_a rated up instead → rule_a first.
        let mut ratings = EloTable::new();
        for _ in 0..2 {
            ratings.play("rule_a", "span", true);
        }
        let result = search.search_with_evidence(buggy, &candidates, &ratings);
        assert_eq!(result[0].entry.rule_id(), "rule_a");
        assert_eq!(result[1].entry.rule_id(), "rule_b");
        assert!(result[0].evidence > result[1].evidence);
    }

    #[test]
    fn search_equal_evidence_preserves_input_order_and_is_deterministic() {
        fn ids<'a>(r: &'a [ScoredCandidate<'a, ClippyFixEntry>]) -> Vec<&'a str> {
            r.iter().map(|s| s.entry.rule_id()).collect()
        }
        // End-to-end fixture (c): equal evidence (both unrated — empty
        // table) → the stable sort preserves input order, and repeated
        // searches are identical.
        let search = RuliologySearch::new();
        let buggy = "fn foo() -> i32 { return 42; }";
        let candidates = vec![
            mock_entry("rule_a", buggy, "fn foo() -> i32 { 42 }"),
            mock_entry("rule_b", buggy, "fn foo() -> i32 { 42 }"),
            mock_entry("rule_c", buggy, "fn foo() -> i32 { 42 }"),
        ];
        let ratings = EloTable::new(); // unrated: all base 1200
        let r1 = search.search_with_evidence(buggy, &candidates, &ratings);
        assert_eq!(
            ids(&r1),
            vec!["rule_a", "rule_b", "rule_c"],
            "equal evidence must preserve input order (stable sort)"
        );
        // Unrated lookups return the exact base constant (no arithmetic) —
        // exact equality IS the unrated contract (the elo.rs test precedent).
        #[allow(clippy::float_cmp)]
        let all_base = r1.iter().all(|s| s.evidence == default_elo());
        assert!(all_base);
        let r2 = search.search_with_evidence(buggy, &candidates, &ratings);
        assert_eq!(ids(&r1), ids(&r2), "search must be deterministic");
        // The unrated search() path behaves identically (base evidence for
        // all — the pre-039 ordering, bit-identical).
        let r3 = search.search(buggy, &candidates);
        assert_eq!(ids(&r1), ids(&r3));
    }

    fn mock_scored(satisfied: bool, complexity: usize) -> ScoredCandidate<'static, ClippyFixEntry> {
        mock_scored_with_evidence(satisfied, complexity, default_elo())
    }

    fn mock_scored_with_evidence(
        satisfied: bool,
        complexity: usize,
        evidence: f32,
    ) -> ScoredCandidate<'static, ClippyFixEntry> {
        // SAFETY: we never read `entry` in the dominates/dominance tests, so
        // we use a leaked static. This is test-only.
        static EMPTY_ENTRY: std::sync::OnceLock<ClippyFixEntry> = std::sync::OnceLock::new();
        let entry = EMPTY_ENTRY.get_or_init(|| ClippyFixEntry {
            lint_name: String::new(),
            lint_group: String::new(),
            buggy_code: String::new(),
            fixed_code: String::new(),
        });
        ScoredCandidate {
            entry,
            lint_satisfied: satisfied,
            ast_delta_size: complexity,
            pareto_rank: 0,
            evidence,
        }
    }

    // ── ast_delta_size tests ────────────────────────────────────────────

    #[test]
    fn ast_delta_identical_code_is_zero() {
        let code = "fn foo() -> i32 { 42 }";
        assert_eq!(ast_delta_size(code, code), 0);
    }

    #[test]
    fn ast_delta_different_code_is_positive() {
        let buggy = "fn foo() -> i32 { return 42; }";
        let fixed = "fn foo() -> i32 { 42 }";
        let delta = ast_delta_size(buggy, fixed);
        assert!(delta > 0, "different code must have positive delta");
    }

    #[test]
    fn ast_delta_unparseable_buggy_returns_max() {
        let buggy = "this is not rust";
        let fixed = "fn foo() -> i32 { 42 }";
        assert_eq!(ast_delta_size(buggy, fixed), usize::MAX);
    }

    #[test]
    fn ast_delta_unparseable_fixed_returns_max() {
        let buggy = "fn foo() -> i32 { 42 }";
        let fixed = "this is not rust";
        assert_eq!(ast_delta_size(buggy, fixed), usize::MAX);
    }

    #[test]
    fn ast_delta_simpler_fix_is_smaller() {
        // Two ways to "fix" the same lint:
        // - Fix A (simple): just remove `return` keyword → smaller delta
        // - Fix B (complex): rewrite as match → larger delta
        let buggy = "fn foo() -> i32 { return 42; }";
        let simple_fix = "fn foo() -> i32 { 42 }";
        let complex_fix = "fn foo() -> i32 { match () { () => 42 } }";
        let simple_delta = ast_delta_size(buggy, simple_fix);
        let complex_delta = ast_delta_size(buggy, complex_fix);
        assert!(
            simple_delta < complex_delta,
            "simple fix (delta={simple_delta}) must be smaller than complex (delta={complex_delta})"
        );
    }

    // ── RuliologySearch integration tests ───────────────────────────────

    #[test]
    fn search_empty_candidates_returns_empty() {
        let search = RuliologySearch::new();
        let empty: [ClippyFixEntry; 0] = [];
        let result = search.search("fn foo() {}", &empty);
        assert!(result.is_empty());
    }

    #[test]
    fn search_single_candidate_returns_it() {
        let search = RuliologySearch::new();
        let entry = mock_entry(
            "needless_return",
            "fn foo() -> i32 { return 42; }",
            "fn foo() -> i32 { 42 }",
        );
        let candidates = vec![entry];
        let result = search.search("fn foo() -> i32 { return 42; }", &candidates);
        assert_eq!(result.len(), 1);
        assert!(result[0].lint_satisfied);
        assert_eq!(result[0].pareto_rank, 0);
    }

    #[test]
    fn best_fix_picks_minimum_complexity_among_pareto_optimal() {
        let search = RuliologySearch::new();
        let buggy = "fn foo() -> i32 { return 42; }";
        let simple_code = "fn foo() -> i32 { 42 }".to_string();
        let complex_code = "fn foo() -> i32 { match () { () => 42 } }".to_string();
        let candidates = vec![
            mock_entry("needless_return", buggy, &complex_code),
            mock_entry("needless_return", buggy, &simple_code),
        ];
        let best = search.best_fix(buggy, &candidates);
        assert!(best.is_some());
        let best = best.unwrap();
        assert_eq!(
            best.fixed_code, simple_code,
            "best fix must be the simpler one"
        );
    }

    #[test]
    fn best_fix_returns_none_when_no_candidate_satisfies() {
        let search = RuliologySearch::new();
        let buggy = "fn foo() -> i32 { return 42; }";
        let candidates = vec![mock_entry(
            "needless_return",
            buggy,
            "this is not valid rust", // unparseable
        )];
        let best = search.best_fix(buggy, &candidates);
        assert!(best.is_none());
    }

    #[test]
    fn pareto_optimal_filters_dominated() {
        let search = RuliologySearch::new();
        let buggy = "fn foo() -> i32 { return 42; }";
        let candidates = vec![
            mock_entry("needless_return", buggy, "fn foo() -> i32 { 42 }"),
            mock_entry(
                "needless_return",
                buggy,
                "fn foo() -> i32 { match () { () => 42 } }",
            ),
        ];
        let front = search.pareto_optimal(buggy, &candidates);
        assert_eq!(
            front.len(),
            1,
            "complex fix dominated by simple fix → only 1 Pareto-optimal"
        );
        assert_eq!(front[0].entry.fixed_code, "fn foo() -> i32 { 42 }");
    }

    #[test]
    fn search_deterministic_same_input_same_output() {
        // G1 determinism: same (buggy, candidates, pruner) → identical ranking.
        let search = RuliologySearch::new();
        let buggy = "fn foo() -> i32 { return 42; }";
        let candidates = vec![
            mock_entry("needless_return", buggy, "fn foo() -> i32 { 42 }"),
            mock_entry(
                "needless_return",
                buggy,
                "fn foo() -> i32 { match () { () => 42 } }",
            ),
        ];

        let r1 = search.search(buggy, &candidates);
        let r2 = search.search(buggy, &candidates);

        assert_eq!(r1.len(), r2.len());
        for (a, b) in r1.iter().zip(r2.iter()) {
            assert_eq!(a.lint_satisfied, b.lint_satisfied);
            assert_eq!(a.ast_delta_size, b.ast_delta_size);
            assert_eq!(a.pareto_rank, b.pareto_rank);
            assert_eq!(a.entry.fixed_code, b.entry.fixed_code);
        }
    }

    // ── Seed corpus integration (G3 no-regression on real data) ─────────

    #[test]
    fn search_over_seed_corpus_lints() {
        let corpus = build_seed_corpus();
        let search = RuliologySearch::new();

        for entry in &corpus.entries {
            let candidates: Vec<ClippyFixEntry> =
                corpus.lookup(&entry.lint_name).cloned().collect();
            let result = search.search(&entry.buggy_code, &candidates);
            assert!(
                !result.is_empty(),
                "search must return results for lint '{}'",
                entry.lint_name
            );
            // The seed corpus has one candidate per lint → must be rank 0.
            assert_eq!(result[0].pareto_rank, 0);
        }
    }

    #[test]
    fn best_fix_over_seed_corpus_finds_valid_fix() {
        let corpus = build_seed_corpus();
        let search = RuliologySearch::new();

        for entry in &corpus.entries {
            let candidates: Vec<ClippyFixEntry> =
                corpus.lookup(&entry.lint_name).cloned().collect();
            let best = search.best_fix(&entry.buggy_code, &candidates);
            assert!(
                best.is_some(),
                "best_fix must return Some for lint '{}'",
                entry.lint_name
            );
        }
    }

    // ── Determinism across multiple lints (G1 extension) ────────────────

    #[test]
    fn search_pareto_rank_consistent_with_pareto_front() {
        // The `pareto_rank == 0` set must equal `pareto_front()`'s output.
        let search = RuliologySearch::new();
        let buggy = "fn foo() -> i32 { return 42; }";
        let candidates = vec![
            mock_entry("needless_return", buggy, "fn foo() -> i32 { 42 }"),
            mock_entry(
                "needless_return",
                buggy,
                "fn foo() -> i32 { match () { () => 42 } }",
            ),
            mock_entry("needless_return", buggy, "fn foo() -> i32 { { 42 } }"),
        ];
        let scored = search.search(buggy, &candidates);

        // Extract payoffs + complexities, run the standalone pareto_front.
        let payoffs: Vec<bool> = scored.iter().map(|s| s.lint_satisfied).collect();
        let complexities: Vec<usize> = scored.iter().map(|s| s.ast_delta_size).collect();
        let front = pareto_front(&payoffs, &complexities);

        // The `pareto_rank == 0` candidates must be a subset of the front.
        // (Not necessarily equal because `search` sorts, but every rank-0
        // candidate must appear in `front`.)
        let rank_zero: Vec<usize> = scored
            .iter()
            .enumerate()
            .filter_map(|(i, s)| if s.pareto_rank == 0 { Some(i) } else { None })
            .collect();

        for idx in &rank_zero {
            assert!(
                front.contains(idx),
                "rank-0 candidate at sorted-index {idx} must be in pareto_front output"
            );
        }
    }
}

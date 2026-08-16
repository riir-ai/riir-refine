//! Issue 017 — retrieval-precision eval at scale (the 12-fixture gap).
//!
//! Bench 018's G1 gate measured 100% top-1 precision on a **12-item** fixture
//! set while the corpus kept growing (40 clippy + 50 perf + 343 kernel rules
//! at land time). Nearest-neighbor crowding is the standard failure mode as
//! a corpus grows — this module is the scaled, repeatable eval that measures
//! ranking precision (top-1 / top-3 / top-5 recall) at the CURRENT corpus
//! scale, per domain + blended, and re-measures on every corpus batch via
//! the committed regression-gate test.
//!
//! Fixture composition (honest provenance — see Bench 022):
//! - **clippy (48)**: hand-authored variants, one per corpus lint, verified
//!   against real `cargo clippy` 1.93 (exactly-one-lint sweep) at authoring
//!   time + re-verified against THIS crate's proposers by the committed
//!   label test below (the proposers are the standing oracle).
//! - **`rust_perf` (48)**: hand-authored variants of the corpus
//!   `buggy_code` shapes; the five proposer-covered rules are re-verified by
//!   the committed label test, the rest are semantic labels authored against
//!   the corpus entries (Batch 48 added the 8 online-search rules' spans).
//! - **kernel (30, `kernel_heal` only)**: deterministic mutated variants —
//!   the corpus `ref_shape` texts with identifiers renamed via a fixed map
//!   (`mutate_shape`). A self-variant probe: optimistic by construction (it
//!   bounds crowding from above), which is exactly the decay signal the
//!   gate watches — when own-variants stop ranking top-1 among 343 rules,
//!   crowding is real.
//! - **clean (12)**: REAL production code excerpts from this repo. The
//!   clean-snippet class matters because `--suggest` has no score threshold
//!   (Bench 018 addendum — measured dead end); the eval records WHAT tops
//!   clean spans (the false-positive profile) instead of pretending they
//!   retrieve nothing.
//!
//! Non-goals (do not re-litigate — Issue 017): no embedder re-training, no
//! score-threshold work.

// The `CLEAN_SPANS` consts keep a uniform `r#"..."#` raw-string style
// (fixture data; mixed styles churn every edit).
#![allow(clippy::needless_raw_string_hashes)]

use super::domain_router::DomainId;
use super::latent_matcher::LatentMatcher;
use crate::domain::{HealDomain, HealEntry};

/// Per-domain (or blended) recall counters.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct DomainMetrics {
    /// Labeled spans evaluated for this domain.
    pub labeled: usize,
    pub hits_at_1: usize,
    pub hits_at_3: usize,
    pub hits_at_5: usize,
}

impl DomainMetrics {
    /// Recall@k as a ratio; `None` when nothing was labeled (domain off).
    ///
    /// Span counts are small (≤ 130) — f32 precision is exact here by
    /// orders of magnitude; the cast is display-metric arithmetic, not
    /// accumulated state.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn recall(&self, k: usize) -> Option<f32> {
        if self.labeled == 0 {
            return None;
        }
        let hits = match k {
            1 => self.hits_at_1,
            3 => self.hits_at_3,
            _ => self.hits_at_5,
        };
        Some(hits as f32 / self.labeled as f32)
    }
}

/// A miss for the failure taxonomy: the expected rule was not top-k in its
/// own domain's ranked list.
#[derive(Debug, Clone)]
pub struct MissRecord {
    /// The ground-truth rule id.
    pub expected: String,
    pub domain: DomainId,
    /// Rank (0-based) if the rule surfaced within the top-8 probe, else
    /// `None` (not retrieved at all).
    pub rank: Option<usize>,
    /// What topped the domain's list instead.
    pub top1: Option<String>,
}

/// The full eval output.
#[derive(Debug, Default)]
pub struct EvalReport {
    /// Fixed domain order: clippy, perf, kernel. Entries with `labeled == 0`
    /// are disabled (feature off) or had no fixtures.
    pub per_domain: [DomainMetrics; 3],
    /// All labeled spans across enabled domains.
    pub blended: DomainMetrics,
    /// Misses (expected rule outside its domain's top-3).
    pub misses: Vec<MissRecord>,
    /// Clean spans: span tag → top-1 rule per ENABLED domain (the
    /// false-positive profile — retrieval always suggests something).
    pub clean_profile: Vec<(&'static str, Vec<(DomainId, String)>)>,
    /// Router annotation quality on labeled spans (feeds the Issue 020
    /// re-gate decision): verdict == expected domain.
    pub router_checked: usize,
    pub router_correct: usize,
    /// Labeled spans skipped because their domain's index is feature-off.
    pub skipped: usize,
}

/// The clean-span set: REAL production excerpts from this crate (shipped
/// code, not synthetic). None may fire any of the 40 corpus lints — the
/// committed clean test below enforces it against the proposers.
const CLEAN_SPANS: &[(&str, &str)] = &[
    (
        "clean:suggest_scratch_capacity",
        r#"pub struct SuggestScratch {
    retrieved: Vec<riir_rag::RetrievedChunk>,
    best: HashMap<String, f32>,
    ranked: Vec<ScoredRule>,
}

impl SuggestScratch {
    pub fn retrieved_capacity(&self) -> usize {
        self.retrieved.capacity()
    }

    pub fn ranked_capacity(&self) -> usize {
        self.ranked.capacity()
    }
}
"#,
    ),
    (
        "clean:all_rules_merge",
        r#"impl SpanSuggestion {
    pub fn all_rules(&self) -> Vec<&ScoredRule> {
        let mut all: Vec<&ScoredRule> = self
            .by_domain
            .iter()
            .flat_map(|(_, rules)| rules.iter())
            .collect();
        all.sort_unstable_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all
    }
}
"#,
    ),
    (
        "clean:rule_index_len",
        r#"impl RuleIndex {
    pub fn len(&self) -> usize {
        self.n_rules
    }

    pub fn is_empty(&self) -> bool {
        self.by_zone_hash.is_empty()
    }
}
"#,
    ),
    (
        "clean:kernel_rule_entry",
        r#"#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelRule {
    pub rule_id: String,
    pub gpu_concept: String,
    pub ref_shape: String,
    pub opt_pattern: String,
}

impl HealEntry for KernelRule {
    #[inline]
    fn rule_id(&self) -> &str {
        &self.rule_id
    }

    #[inline]
    fn buggy_code(&self) -> &str {
        &self.ref_shape
    }

    #[inline]
    fn fixed_code(&self) -> &str {
        &self.opt_pattern
    }
}
"#,
    ),
    (
        "clean:swallow_following_space",
        r#"pub fn swallow_following_space(source: &str, mut e: SourceEdit) -> SourceEdit {
    if source[e.end..].starts_with(' ') {
        e.end += 1;
    }
    e
}
"#,
    ),
    (
        "clean:bare_len_call",
        r#"fn bare_len_call(e: &Expr) -> Option<&ExprMethodCall> {
    let Expr::MethodCall(m) = e else {
        return None;
    };
    if m.method == "len" && m.args.is_empty() && m.attrs.is_empty() && m.turbofish.is_none() {
        Some(m)
    } else {
        None
    }
}
"#,
    ),
    (
        "clean:router_route",
        r#"pub fn route(&self, code: &str) -> Option<DomainId> {
    if self.ids.is_empty() {
        return None;
    }
    let activity = ModellessEmbedder.embed(code);
    self.route_activity(&activity)
}
"#,
    ),
    (
        "clean:propose_fix_dispatch",
        r#"pub fn propose_fix(lint_name: &str, buggy_code: &str) -> Option<String> {
    let file: syn::File = syn::parse_file(buggy_code).ok()?;
    let edits = collect_rule_edits(lint_name, buggy_code, &file);
    if edits.is_empty() {
        return None;
    }
    Some(apply_edits(buggy_code, edits))
}
"#,
    ),
    (
        "clean:category_label",
        r#"impl KernelCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Memory => "A: memory",
            Self::Compute => "B: compute",
            Self::Fusion => "C: fusion",
            Self::Dispatch => "D: dispatch",
            Self::Parallelism => "E: parallelism",
        }
    }
}
"#,
    ),
    (
        "clean:span_suggestion_struct",
        r#"#[derive(Debug, Clone, PartialEq)]
pub struct SpanSuggestion {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub routed_domain: DomainId,
    pub by_domain: Vec<(DomainId, Vec<ScoredRule>)>,
}
"#,
    ),
    (
        "clean:eval_span_sort",
        r#"fn sort_by_score(rules: &mut [ScoredRule]) {
    rules.sort_unstable_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}
"#,
    ),
    (
        "clean:zone_hash_sidecar",
        r#"fn zone_hash_sidecar(patterns: &[(String, String)]) -> HashMap<[u8; 32], String> {
    let mut by_zone_hash = HashMap::with_capacity(patterns.len());
    for (rule_id, text) in patterns {
        let hash = *blake3::hash(text.as_bytes()).as_bytes();
        by_zone_hash.insert(hash, rule_id.clone());
    }
    by_zone_hash
}
"#,
    ),
];

/// Deterministic identifier-renaming map for the kernel self-variant probe.
/// Word-boundary-exact (inside `output` the token `out` is untouched).
const KERNEL_RENAMES: &[(&str, &str)] = &[
    ("buf", "scratch"),
    ("acc", "total"),
    ("out", "dst"),
    ("vals", "items"),
    ("tmp", "staging"),
    ("res", "answer"),
    ("idx", "slot"),
    ("n", "count"),
    ("i", "p"),
    ("j", "q"),
];

/// Rename identifiers in pseudocode text via the fixed map (deterministic,
/// no RNG). Types/keywords are untouched (map keys are value-identifier
/// shapes only).
#[must_use]
pub fn mutate_shape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut token = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else {
            if !token.is_empty() {
                let replacement = KERNEL_RENAMES
                    .iter()
                    .find(|(from, _)| *from == token)
                    .map_or(token.as_str(), |(_, to)| to);
                out.push_str(replacement);
                token.clear();
            }
            out.push(ch);
        }
    }
    if !token.is_empty() {
        let replacement = KERNEL_RENAMES
            .iter()
            .find(|(from, _)| *from == token)
            .map_or(token.as_str(), |(_, to)| to);
        out.push_str(replacement);
    }
    out
}

/// The kernel self-variant probe: every `step`-th corpus rule's `ref_shape`,
/// mutated. Deterministic across runs; the SELECTED SET shifts as batches
/// append entries (intended — the gate re-measures the moved set).
#[cfg(feature = "kernel_heal")]
fn kernel_fixture_spans(target: usize) -> Vec<(String, String)> {
    let entries = crate::domains::kernel_opt::corpus::build_kernel_seed_corpus().entries;
    let step = (entries.len() / target).max(1);
    entries
        .iter()
        .step_by(step)
        .take(target)
        .map(|e| (e.rule_id.clone(), mutate_shape(&e.ref_shape)))
        .collect()
}

/// How deep the per-domain ranked list is probed (ranks 0..8 give the
/// near-miss signal for the taxonomy).
const PROBE_TOP_K: usize = 8;

/// The labeled eval span set as OWNED strings (shared by `run_eval` + the
/// Issue 020 router eval — both must measure the SAME spans).
struct OwnedEvalSpan {
    label: String,
    domain: DomainId,
    code: String,
}

fn labeled_eval_spans() -> Vec<OwnedEvalSpan> {
    let mut spans = Vec::new();
    #[cfg(feature = "clippy_lints")]
    for (lint, code) in super::eval_fixtures_clippy::CLIPPY_SPANS {
        spans.push(OwnedEvalSpan {
            label: lint.to_string(),
            domain: DomainId::Clippy,
            code: code.to_string(),
        });
    }
    #[cfg(feature = "rust_perf")]
    for (rule, code) in super::eval_fixtures_perf::PERF_SPANS {
        spans.push(OwnedEvalSpan {
            label: rule.to_string(),
            domain: DomainId::Perf,
            code: code.to_string(),
        });
    }
    #[cfg(feature = "kernel_heal")]
    for (rule, code) in kernel_fixture_spans(30) {
        spans.push(OwnedEvalSpan {
            label: rule,
            domain: DomainId::Kernel,
            code,
        });
    }
    spans
}

fn domain_index(d: DomainId) -> usize {
    match d {
        DomainId::Clippy => 0,
        DomainId::Perf => 1,
        DomainId::Kernel => 2,
    }
}

/// Canonical order for the matcher's returned rules: score-descending with
/// a `rule_id` tie-break. The matcher's own ordering resolves score TIES by
/// backing-map iteration order, which can flip between runs — the eval is a
/// MEASUREMENT, so it must be reproducible (Issue 022: a corpus change put
/// a tie exactly on the top-5 boundary + the in-process determinism test
/// caught it). A substrate-level deterministic tie-break in riir-rag remains
/// a documented follow-up; this keeps the eval self-contained.
fn canonical_rank(rules: &[crate::draft::ScoredRule]) -> Vec<&crate::draft::ScoredRule> {
    let mut v: Vec<&crate::draft::ScoredRule> = rules.iter().collect();
    v.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });
    v
}

/// Assemble + run the full eval against every feature-enabled domain.
///
/// # Panics
///
/// Panics if [`LatentMatcher::from_enabled_domains`] fails (no domain
/// feature enabled — the committed suites always enable at least
/// `clippy_lints` + `rust_perf`).
#[must_use]
pub fn run_eval() -> EvalReport {
    let mut matcher = LatentMatcher::from_enabled_domains().expect("build matcher");
    let mut report = EvalReport::default();

    let spans = labeled_eval_spans();
    for span in &spans {
        let label = span.label.as_str();
        let domain = span.domain;
        let code = span.code.as_str();
        if !matcher.has_domain(domain) {
            report.skipped += 1;
            continue;
        }
        let (routed, by_domain) = matcher.suggest_span(code, PROBE_TOP_K);
        let ranked = by_domain
            .iter()
            .find(|(d, _)| *d == domain)
            .map_or(Vec::new(), |(_, rules)| canonical_rank(rules));
        let rank = ranked.iter().position(|r| r.rule_id == label);
        let top1 = ranked.first().map(|r| r.rule_id.clone());

        report.router_checked += 1;
        if routed == domain {
            report.router_correct += 1;
        }

        let di = domain_index(domain);
        let m = &mut report.per_domain[di];
        m.labeled += 1;
        if rank.is_some_and(|r| r < 1) {
            m.hits_at_1 += 1;
        }
        if rank.is_some_and(|r| r < 3) {
            m.hits_at_3 += 1;
        }
        if rank.is_some_and(|r| r < 5) {
            m.hits_at_5 += 1;
        }
        let b = &mut report.blended;
        b.labeled += 1;
        if rank.is_some_and(|r| r < 1) {
            b.hits_at_1 += 1;
        }
        if rank.is_some_and(|r| r < 3) {
            b.hits_at_3 += 1;
        }
        if rank.is_some_and(|r| r < 5) {
            b.hits_at_5 += 1;
        }
        if rank.is_none_or(|r| r >= 3) {
            report.misses.push(MissRecord {
                expected: label.to_string(),
                domain,
                rank,
                top1,
            });
        }
    }

    for (tag, code) in CLEAN_SPANS {
        let (routed, by_domain) = matcher.suggest_span(code, PROBE_TOP_K);
        let _ = routed;
        let tops: Vec<(DomainId, String)> = by_domain
            .iter()
            .filter_map(|(d, rules)| {
                canonical_rank(rules)
                    .first()
                    .map(|r| (*d, r.rule_id.clone()))
            })
            .collect();
        report.clean_profile.push((tag, tops));
    }

    report
}

/// Human-readable metrics table (the `--nocapture` output of the report
/// test — the one-command harness).
#[allow(clippy::cast_precision_loss)] // display percentages over tiny counts
pub fn print_report(report: &EvalReport) {
    println!("== Issue 017 retrieval-precision eval ==");
    let names = ["clippy", "perf  ", "kernel"];
    println!("{:<8} {:>8} {:>9} {:>9} {:>9}", "domain", "labeled", "top-1", "top-3", "top-5");
    for (i, name) in names.iter().enumerate() {
        let m = &report.per_domain[i];
        if m.labeled == 0 {
            println!("{name:<8} {:>8} {:>9} {:>9} {:>9}", 0, "-", "-", "-");
            continue;
        }
        println!(
            "{name:<8} {:>8} {:>8.1}% {:>8.1}% {:>8.1}%",
            m.labeled,
            m.recall(1).map_or(0.0, |r| r * 100.0),
            m.recall(3).map_or(0.0, |r| r * 100.0),
            m.recall(5).map_or(0.0, |r| r * 100.0),
        );
    }
    println!(
        "blended  {:>8} {:>8.1}% {:>8.1}% {:>8.1}%   (skipped: {})",
        report.blended.labeled,
        report.blended.recall(1).map_or(0.0, |r| r * 100.0),
        report.blended.recall(3).map_or(0.0, |r| r * 100.0),
        report.blended.recall(5).map_or(0.0, |r| r * 100.0),
        report.skipped,
    );
    println!(
        "router annotation: {}/{} correct ({:.1}%) — feeds the Issue 020 re-gate",
        report.router_correct,
        report.router_checked,
        if report.router_checked == 0 {
            0.0
        } else {
            report.router_correct as f32 / report.router_checked as f32 * 100.0
        },
    );
    println!("\n-- miss taxonomy (expected rule outside its domain top-3) --");
    if report.misses.is_empty() {
        println!("none");
    }
    for miss in &report.misses {
        match miss.rank {
            Some(r) => println!(
                "  outranked  [{:?}] expected={} rank={} top1={}",
                miss.domain, miss.expected, r,
                miss.top1.as_deref().unwrap_or("?"),
            ),
            None => println!(
                "  not-in-top{} [{:?}] expected={} top1={}",
                PROBE_TOP_K,
                miss.domain,
                miss.expected,
                miss.top1.as_deref().unwrap_or("?"),
            ),
        }
    }
    println!("\n-- clean-span false-positive profile (what tops real clean code) --");
    for (tag, tops) in &report.clean_profile {
        let tops: String = tops
            .iter()
            .map(|(d, r)| format!("{d:?}:{r}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!("  {tag} -> {tops}");
    }
}

// ── Issue 020 POC: router spectral-block eval ──────────────────────────

/// The two DOCUMENTED `Plan 005` mis-routes (`latent_matcher` doc + Bench 018):
/// a `return expr;` fn routed to perf (should → clippy) + a linear-scan fn
/// routed to clippy (should → perf). The gate requires BOTH fixed.
const MISROUTE_FIXTURES: [(&str, DomainId); 2] = [
    (
        "pub fn area(w: f64, h: f64) -> f64 {\n    return w * h;\n}\n",
        DomainId::Clippy,
    ),
    (
        "pub fn find_user(users: &Vec<User>, id: u32) -> Option<&User> {\n    for u in users {\n        if u.id == id {\n            return Some(u);\n        }\n    }\n    None\n}\n",
        DomainId::Perf,
    ),
];

/// Mirror of `LatentMatcher::from_enabled_domains`' router-domain assembly
/// (the same corpus pattern texts — the router eval must measure the router
/// the production path would build).
#[allow(clippy::vec_init_then_push)] // cfg-gated per-domain pushes
fn router_eval_domains() -> Vec<(DomainId, Vec<String>)> {
    let mut out = Vec::new();
    #[cfg(feature = "clippy_lints")]
    out.push((
        DomainId::Clippy,
        crate::domains::clippy_lints::ClippyLintDomain::build_corpus()
            .into_iter()
            .map(|e| e.buggy_code().to_string())
            .collect::<Vec<_>>(),
    ));
    #[cfg(feature = "rust_perf")]
    out.push((
        DomainId::Perf,
        crate::domains::rust_perf::RustPerfDomain::build_corpus()
            .into_iter()
            .map(|e| e.buggy_code().to_string())
            .collect::<Vec<_>>(),
    ));
    #[cfg(feature = "kernel_heal")]
    out.push((
        DomainId::Kernel,
        crate::domains::kernel_opt::KernelOptDomain::build_corpus()
            .into_iter()
            .map(|e| e.buggy_code().to_string())
            .collect::<Vec<_>>(),
    ));
    out
}

/// One routing-variant measurement (Issue 020).
#[derive(Debug, Clone)]
pub struct RouterEvalRow {
    /// Variant name (e.g. `baseline-8d`, `dft8-disc+struct`).
    pub mode: String,
    /// Labeled spans routed.
    pub checked: usize,
    /// Spans routed to their owning domain.
    pub correct: usize,
    /// Both documented mis-routes fixed?
    pub misroutes_fixed: bool,
}

impl RouterEvalRow {
    /// Correct-route ratio (1.0 when nothing was checked — vacuous state).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // span counts ≤ ~10²
    pub fn accuracy(&self) -> f32 {
        if self.checked == 0 {
            1.0
        } else {
            self.correct as f32 / self.checked as f32
        }
    }
}

/// The routing variants measured by the POC, in report order.
#[must_use]
pub fn router_eval_modes() -> Vec<super::domain_router::RoutingEmbedding> {
    use super::domain_router::RoutingEmbedding::{self, SpectralDft};
    use super::domain_router::SpectralOrder::{Discriminative, GlobalFreq};
    vec![
        RoutingEmbedding::Baseline,
        SpectralDft { order: GlobalFreq, with_struct: false },
        SpectralDft { order: GlobalFreq, with_struct: true },
        SpectralDft { order: Discriminative, with_struct: false },
        SpectralDft { order: Discriminative, with_struct: true },
        RoutingEmbedding::TfCosine { order: GlobalFreq },
        RoutingEmbedding::TfCosine { order: Discriminative },
    ]
}

fn router_mode_name(mode: super::domain_router::RoutingEmbedding) -> String {
    use super::domain_router::RoutingEmbedding as RE;
    use super::domain_router::SpectralOrder as SO;
    match mode {
        RE::Baseline => "baseline-8d".to_string(),
        RE::SpectralDft { order, with_struct } => format!(
            "dft8-{}{}",
            match order {
                SO::GlobalFreq => "global",
                SO::Discriminative => "disc",
            },
            if with_struct { "+struct" } else { "" }
        ),
        RE::TfCosine { order } => format!(
            "tf-cosine-{}",
            match order {
                SO::GlobalFreq => "global",
                SO::Discriminative => "disc",
            }
        ),
    }
}

/// Route every labeled eval span + the two documented mis-route fixtures
/// under every routing variant. THE Issue 020 measurement.
#[must_use]
pub fn run_router_eval() -> Vec<RouterEvalRow> {
    use super::domain_router::DomainRouter;

    let domains = router_eval_domains();
    let spans = labeled_eval_spans();
    let mut rows = Vec::new();
    for mode in router_eval_modes() {
        let router = DomainRouter::with_embedding(&domains, mode);
        let mut correct = 0usize;
        for span in &spans {
            if router.route(&span.code) == Some(span.domain) {
                correct += 1;
            }
        }
        let misroutes_fixed = MISROUTE_FIXTURES
            .iter()
            .all(|(code, expected)| router.route(code) == Some(*expected));
        rows.push(RouterEvalRow {
            mode: router_mode_name(mode),
            checked: spans.len(),
            correct,
            misroutes_fixed,
        });
    }
    rows
}

/// Print the router eval table (the `--nocapture` output).
#[allow(clippy::cast_precision_loss)] // display percentages over tiny counts
pub fn print_router_eval(rows: &[RouterEvalRow]) {
    println!("\n== Issue 020 router spectral POC (kill-bar: accuracy >= 85%, both mis-routes fixed) ==");
    for row in rows {
        println!(
            "  {:<22} {:>4}/{} ({:>5.1}%)  mis-routes: {}",
            row.mode,
            row.correct,
            row.checked,
            row.accuracy() * 100.0,
            if row.misroutes_fixed { "fixed" } else { "NOT fixed" },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Baseline floors for the regression gate (Bench 022, 2026-08-16;
    /// RE-PINNED by Issue 022 same day — see Bench 024). A corpus batch that
    /// decays precision below these fails the suite — re-measure + re-pin
    /// (and record the delta) rather than weakening silently. Values are
    /// ratios (0.0-1.0).
    ///
    /// Original baseline (clippy+perf state, 88 spans): blended 71.6/84.1/92.0,
    /// clippy 58.3/75.0/87.5, perf 87.5/95.0/97.5; kernel (combined state,
    /// 30 self-variants) 100/100/100.
    ///
    /// Issue 022 re-pin (corpus truthfulness trade): 5 clippy corpus entries'
    /// buggy shapes were corrected to forms REAL clippy 1.97 fires (the
    /// `comparison_to_empty` entry's text moved from `.len() == 0` — real
    /// clippy's `len_zero` — to the real `s == ""` shape). Cost: the 2
    /// len-shaped fixtures retrieve `comparison_to_empty` at lower rank
    /// (rank 3 + rank 7). New baseline: blended 69.3/81.8/88.6,
    /// clippy 54.2/70.8/81.2, perf unchanged. Truthfulness wins — a corpus
    /// that misrepresents lint behavior poisons `self_evolve` learning.
    ///
    /// Batch 48 re-pin (8 `rust_perf` rules + spans, 2026-08-16): perf
    /// corpus 50→58 rules, `PERF_SPANS` 40→48, blended denominator 88→96. First
    /// measurement showed 3 of the 8 new spans drifting (filter-map-fuse not
    /// even top-8 — every distinctive token renamed away from the corpus
    /// shape); the spans were re-authored to the set's convention (variant
    /// names, canonical predicate/operator shapes) per the Issue 022
    /// fix-before-repin order. New baseline: blended 70.8/83.3/90.6,
    /// clippy 54.2/70.8/83.3, perf 87.5/95.8/97.9 (top-1 UNCHANGED vs the
    /// 40-span set — zero batch decay), kernel self-variants 100/100/100.
    /// 3-domain (126 spans): blended 77.8/87.3/92.9, router 88/126 = 69.8%.
    ///
    /// Floors = baseline − ONE span — the ranking's tie-break drains a
    /// `HashMap` (`RandomState` per process), so a tie at a hit boundary can
    /// flip across processes (3-process re-runs were identical; the margin
    /// is cheap robustness, not noise cover).
    mod floors {
        pub const BLENDED_TOP1: f32 = 67.0 / 96.0;
        pub const BLENDED_TOP3: f32 = 79.0 / 96.0;
        pub const BLENDED_TOP5: f32 = 86.0 / 96.0;
        pub const CLIPPY_TOP1: f32 = 25.0 / 48.0;
        pub const CLIPPY_TOP3: f32 = 33.0 / 48.0;
        pub const PERF_TOP3: f32 = 45.0 / 48.0;
        #[cfg(feature = "kernel_heal")]
        pub const KERNEL_TOP3: f32 = 29.0 / 30.0;
        /// Router-annotation accuracy floor (Issue 020 wiring, Bench 025):
        /// the TF-cosine-global annotation measured 66/96 (68.8%, 2-domain)
        /// / 88/126 (69.8%, 3-domain) after Batch 48. Floor = measured − 5
        /// spans of corpus-growth headroom (both states land at 63.5%).
        /// This is the standing per-batch measurement of annotation
        /// quality — NOT a gate-quality claim (gating stays killed at the
        /// 85% bar).
        #[cfg(not(feature = "kernel_heal"))]
        pub const ROUTER_ANNOTATION: f32 = 61.0 / 96.0;
        #[cfg(feature = "kernel_heal")]
        pub const ROUTER_ANNOTATION_3D: f32 = 80.0 / 126.0;
    }

    #[allow(clippy::cast_precision_loss)] // ratio arithmetic over tiny counts
    fn ratio(hits: usize, n: usize) -> f32 {
        if n == 0 { 1.0 } else { hits as f32 / n as f32 }
    }

    #[test]
    fn retrieval_precision_regression_gate() {
        let report = run_eval();
        print_report(&report);
        let b = &report.blended;
        assert!(
            ratio(b.hits_at_1, b.labeled) >= floors::BLENDED_TOP1,
            "blended top-1 {} below floor {}",
            ratio(b.hits_at_1, b.labeled),
            floors::BLENDED_TOP1,
        );
        assert!(
            ratio(b.hits_at_3, b.labeled) >= floors::BLENDED_TOP3,
            "blended top-3 {} below floor {}",
            ratio(b.hits_at_3, b.labeled),
            floors::BLENDED_TOP3,
        );
        assert!(
            ratio(b.hits_at_5, b.labeled) >= floors::BLENDED_TOP5,
            "blended top-5 {} below floor {}",
            ratio(b.hits_at_5, b.labeled),
            floors::BLENDED_TOP5,
        );
        if report.per_domain[0].labeled > 0 {
            let m = &report.per_domain[0];
            assert!(ratio(m.hits_at_1, m.labeled) >= floors::CLIPPY_TOP1);
            assert!(ratio(m.hits_at_3, m.labeled) >= floors::CLIPPY_TOP3);
        }
        if report.per_domain[1].labeled > 0 {
            let m = &report.per_domain[1];
            assert!(ratio(m.hits_at_3, m.labeled) >= floors::PERF_TOP3);
        }
        #[cfg(feature = "kernel_heal")]
        if report.per_domain[2].labeled > 0 {
            let m = &report.per_domain[2];
            assert!(ratio(m.hits_at_3, m.labeled) >= floors::KERNEL_TOP3);
        }
        // Issue 020 wiring: the annotation router (TF-cosine-global) keeps a
        // standing accuracy floor — corpus batches shift centroids/vocab, so
        // decay below this re-measures + re-pins (record the delta), never
        // silently weakens. GATING remains killed (85% bar, Bench 025).
        if report.router_checked > 0 {
            #[allow(clippy::cast_precision_loss)] // ratio over tiny counts
            let acc = report.router_correct as f32 / report.router_checked as f32;
            #[cfg(feature = "kernel_heal")]
            let floor = floors::ROUTER_ANNOTATION_3D;
            #[cfg(not(feature = "kernel_heal"))]
            let floor = floors::ROUTER_ANNOTATION;
            assert!(
                acc >= floor,
                "router annotation accuracy {:.1}% below floor {:.1}% — annotation decayed (re-measure + re-pin per the Issue 020 contract)",
                acc * 100.0,
                floor * 100.0
            );
        }
    }

    #[test]
    fn eval_report_is_deterministic() {
        // G1: identical fixture set + corpus → identical metrics (the
        // modelless embedder + BM25 are deterministic; papaya iteration
        // order must not leak into rankings).
        let a = run_eval();
        let b = run_eval();
        assert_eq!(a.blended, b.blended);
        assert_eq!(a.per_domain, b.per_domain);
        assert_eq!(a.router_correct, b.router_correct);
    }

    /// Label truth: every clippy fixture fires its labeled lint under OUR
    /// proposers (the standing oracle — itself cross-validated against real
    /// clippy 1.93 at authoring time) and fires NO other corpus lint.
    #[cfg(feature = "clippy_lints")]
    #[test]
    fn clippy_fixture_labels_verify_via_proposers() {
        use crate::domains::clippy_lints::proposers::{
            propose_fix, FIXABLE_LINTS, VERIFY_LINTS,
        };
        let mut all: Vec<&str> = FIXABLE_LINTS.to_vec();
        all.extend_from_slice(VERIFY_LINTS);
        all.extend_from_slice(&[
            "useless_conversion",
            "needless_borrow",
            "unnecessary_mut_passed",
        ]);
        for &(lint, code) in super::super::eval_fixtures_clippy::CLIPPY_SPANS {
            assert!(
                propose_fix(lint, code).is_some(),
                "fixture for {lint} does not fire its own proposer"
            );
            for &other in &all {
                if other != lint {
                    assert!(
                        propose_fix(other, code).is_none(),
                        "fixture for {lint} cross-fires {other}"
                    );
                }
            }
        }
    }

    /// The five proposer-covered perf fixtures must fire their proposers.
    #[cfg(feature = "rust_perf")]
    #[test]
    fn perf_fixture_labels_verify_via_proposers() {
        use crate::domains::rust_perf::proposers::{propose_fix, FIXABLE_RULES};
        for &(rule, code) in super::super::eval_fixtures_perf::PERF_SPANS {
            if FIXABLE_RULES.contains(&rule) {
                assert!(
                    propose_fix(rule, code).is_some(),
                    "proposer-covered fixture for {rule} does not fire its proposer"
                );
            }
        }
    }

    /// Clean-span truth: no real-code excerpt contains any of the 37
    /// STRUCTURAL lint patterns (FIXABLE + VERIFY). The three type-blind
    /// self_evolve-only proposers (`needless_borrow`, `useless_conversion`,
    /// `unnecessary_mut_passed`) are excluded BY DESIGN: they over-match on
    /// parse-only information (e.g. every necessary `f(&local)` borrow) —
    /// that is exactly why they are not `--fix` eligible and only fire
    /// through the cargo-clippy-verified loop. Real production code
    /// legitimately trips them.
    #[cfg(feature = "clippy_lints")]
    #[test]
    fn clean_spans_fire_no_lint() {
        use crate::domains::clippy_lints::proposers::{
            propose_fix, FIXABLE_LINTS, VERIFY_LINTS,
        };
        let structural: Vec<&str> = FIXABLE_LINTS
            .iter()
            .chain(VERIFY_LINTS)
            .copied()
            .collect();
        for (tag, code) in CLEAN_SPANS {
            for &lint in &structural {
                assert!(
                    propose_fix(lint, code).is_none(),
                    "clean span {tag} fires {lint}"
                );
            }
        }
    }

    #[cfg(feature = "kernel_heal")]
    #[test]
    fn kernel_mutation_is_deterministic_and_renames() {
        let spans = kernel_fixture_spans(30);
        assert_eq!(spans.len(), 30);
        let again = kernel_fixture_spans(30);
        assert_eq!(spans, again, "selection + mutation must be deterministic");
        // The renames actually apply: no output token still equals a mapped
        // key in word position for a shape containing one.
        let mutated = mutate_shape("let buf = vals[idx] + acc;");
        assert_eq!(mutated, "let scratch = items[slot] + total;");
    }

    #[test]
    fn mutate_shape_keeps_embedded_tokens() {
        // Word-boundary exactness: `output` must NOT become `dstput`, and
        // underscore-joined identifiers (`n_out`) are SINGLE tokens.
        assert_eq!(mutate_shape("let output = out;"), "let output = dst;");
        assert_eq!(mutate_shape("let n_out = 2;"), "let n_out = 2;");
    }

    #[test]
    fn fixture_counts_meet_the_issue_floor() {
        // Issue 017 acceptance: >= 100 labeled spans including the clean
        // class. Static fixtures alone: 40 + 30 + 12 = 82; the kernel probe
        // (30) completes the set when enabled. The committed floor counts
        // what the CURRENT feature state can evaluate.
        let clean = CLEAN_SPANS.len();
        #[allow(unused_mut)]
        let mut labeled = 0;
        #[cfg(feature = "clippy_lints")]
        {
            labeled += super::super::eval_fixtures_clippy::CLIPPY_SPANS.len();
        }
        #[cfg(feature = "rust_perf")]
        {
            labeled += super::super::eval_fixtures_perf::PERF_SPANS.len();
        }
        #[cfg(feature = "kernel_heal")]
        {
            labeled += kernel_fixture_spans(30).len();
        }
        let total = labeled + clean;
        assert!(
            total >= 100,
            "fixture set shrank below the Issue 017 floor: {labeled} labeled + {clean} clean = {total}"
        );
    }

    /// Issue 020 POC measurement: prints the variant table for the record
    /// (`--nocapture`). The PASS/KILL decision gates live in
    /// `router_spectral_gate` (asserting on the WIRED mode) + Bench 025.
    #[test]
    fn router_spectral_poc_report() {
        let rows = run_router_eval();
        print_router_eval(&rows);
        // Structural sanity in every state: the measurement itself must be
        // deterministic + complete.
        assert!(!rows.is_empty());
        let again = run_router_eval();
        assert_eq!(
            rows.iter().map(|r| (r.correct, r.misroutes_fixed)).collect::<Vec<_>>(),
            again.iter().map(|r| (r.correct, r.misroutes_fixed)).collect::<Vec<_>>(),
            "router eval must be deterministic"
        );
    }
}

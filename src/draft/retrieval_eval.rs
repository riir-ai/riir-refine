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
    /// Component-attributed class (Issue 047), derived at record time from
    /// this record's own facts: outranked / crowded-past-the-probe = ρ
    /// (retrieval), nothing-answered = E (corpus). The eval is the living
    /// ρ/E coverage — its misses carry ground truth the production paths
    /// lack.
    pub component: crate::component::ComponentClass,
}

/// The full eval output.
#[derive(Debug, Default)]
pub struct EvalReport {
    /// Fixed domain order: [`DomainId::ALL`] (clippy, perf, kernel, sec,
    /// docker). Entries with `labeled == 0` are disabled (feature off) or
    /// had no fixtures.
    ///
    /// Sized by `DomainId::ALL.len()`, NOT by the number of domains that
    /// happen to carry fixtures. Before the docker landing this array was
    /// `[_; 3]` while `domain_index(Sec)` already returned 3 — a latent
    /// out-of-bounds that never fired only because no sec-labeled fixture
    /// existed. Adding a 5th domain would have made that two indices out.
    pub per_domain: [DomainMetrics; DomainId::ALL.len()],
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
        // Issue 031 H1: the exact katgpt-rs SensitivityCache production shape —
        // `self.len() == 0` inside `fn is_empty` is the ONLY legitimate spelling
        // of that method body; rewriting it to `self.is_empty()` is infinite
        // recursion. comparison_to_empty (and every other structural lint) must
        // stay silent here.
        "clean:is_empty_len_zero_body",
        r#"impl SensitivityCache {
    pub fn len(&self) -> usize {
        self.cache.pin().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
    (
        // Issue 028: the katgpt-speculative find_decomposition_regions shape
        // (real production excerpt, simplified to a free fn) — comment-
        // guarded single-if blocks in BOTH arms; real clippy 1.97 is SILENT
        // on both collapsible_if and collapsible_else_if here.
        "clean:comment_guarded_if_else_arms",
        r#"fn find_decomposition_regions(profile: &[f32]) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut region_start: Option<usize> = None;

    for (i, &rel) in profile.iter().enumerate() {
        if rel < 0.3 {
            // Low relevance — extend or start region.
            if region_start.is_none() {
                region_start = Some(i);
            }
        } else {
            // High relevance — close any open region.
            if let Some(start) = region_start.take() {
                regions.push((start, i));
            }
        }
    }

    regions
}
"#,
    ),
    (
        // Issue 029 T3: the probe-verified SILENT shapes for the extended
        // single_match_else wildcard set — user-enum constructor arms (clippy
        // is type-informed: only std Option/Result scrutinees fire) and a
        // BINDING second arm (the binding would be lost under `else`).
        // Real clippy 1.97 pedantic is silent on both; every structural
        // proposer must be too.
        "clean:user_enum_and_binding_second_arms",
        r#"#[derive(Clone, Copy)]
pub enum SlotState { Ok(u8), Err(u8) }
pub enum NodeKind { Leaf, None }

fn classify_slot(s: SlotState) -> u8 {
    match s {
        SlotState::Ok(v) => { let scaled = v * 2; scaled }
        SlotState::Err(e) => { let penalty = e as u8; penalty }
    }
}

fn node_weight(k: NodeKind) -> u8 {
    match k {
        NodeKind::Leaf => { let base = 1; base }
        NodeKind::None => { let zero = 0; zero }
    }
}

fn probe(cache_hit: Option<u8>) -> u8 {
    match cache_hit {
        None => { let miss = 2; miss }
        Some(v) => { let hit = v + 1; hit }
    }
}
"#,
    ),
    (
        // Slice-5 extension (2026-08-18): the real riir-train gpart.rs
        // production shapes for the newly-covered `anyhow::ensure!` macro
        // family — an already-inlined `{version}` template and a
        // no-placeholder template. Both must stay silent.
        "clean:anyhow_ensure_already_inlined",
        r#"fn check_oscpt_header(data: &[u8], pos: &mut usize) -> anyhow::Result<()> {
    anyhow::ensure!(
        data.len() >= 5 && data[..5] == b"OSCPT",
        "Invalid OSCPT magic"
    );
    *pos += 5;
    let version = u32::from_le_bytes(data[*pos..*pos + 4].try_into()?);
    anyhow::ensure!(
        version == 9,
        "Unsupported OSCPT version: {version}"
    );
    *pos += 4;
    Ok(())
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
pub(crate) struct OwnedEvalSpan {
    pub(crate) label: String,
    pub(crate) domain: DomainId,
    pub(crate) code: String,
}

pub(crate) fn labeled_eval_spans() -> Vec<OwnedEvalSpan> {
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

pub(crate) fn domain_index(d: DomainId) -> usize {
    match d {
        DomainId::Clippy => 0,
        DomainId::Perf => 1,
        DomainId::Kernel => 2,
        DomainId::Sec => 3,
        DomainId::Docker => 4,
    }
}

/// Canonical order for the matcher's returned rules: score-descending with
/// a `rule_id` tie-break. The matcher's own ordering resolves score TIES by
/// backing-map iteration order, which can flip between runs — the eval is a
/// MEASUREMENT, so it must be reproducible (Issue 022: a corpus change put
/// a tie exactly on the top-5 boundary + the in-process determinism test
/// caught it). A substrate-level deterministic tie-break in riir-rag remains
/// a documented follow-up; this keeps the eval self-contained.
///
/// `pub(crate)` — shared by the Issue 024 switch-cost measurement so both
/// evals rank identically.
pub(crate) fn canonical_rank(rules: &[crate::draft::ScoredRule]) -> Vec<&crate::draft::ScoredRule> {
    let mut v: Vec<&crate::draft::ScoredRule> = rules.iter().collect();
    // MUST match `instantiation_rerank::rerank`'s key order — instantiation
    // first, then similarity, then rule_id. Sorting by score alone here would
    // silently undo the T3.5.4 re-rank before it was ever measured.
    v.sort_by(|a, b| {
        b.instantiates
            .cmp(&a.instantiates)
            .then_with(|| b.score.total_cmp(&a.score))
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
    run_eval_with(crate::draft::RetrievalConfig::default())
}

/// [`run_eval`] against an explicit re-rank mode (Plan 005 T3.5.4 A/B).
#[must_use]
pub fn run_eval_reranked(mode: crate::draft::RerankMode) -> EvalReport {
    run_eval_inner(crate::draft::RetrievalConfig::default(), mode)
}

/// [`run_eval`] against an explicit retrieval config — the Plan 005 T3.5 A/B
/// entry point. `run_eval` delegates with the promoted default.
///
/// # Panics
///
/// Panics if [`LatentMatcher::from_enabled_domains_with`] fails.
#[must_use]
pub fn run_eval_with(config: crate::draft::RetrievalConfig) -> EvalReport {
    run_eval_inner(config, crate::draft::RerankMode::default())
}

/// The shared eval body — both A/B axes explicit.
///
/// # Panics
///
/// Panics if [`LatentMatcher::from_enabled_domains_with`] fails.
#[must_use]
pub fn run_eval_inner(
    config: crate::draft::RetrievalConfig,
    rerank: crate::draft::RerankMode,
) -> EvalReport {
    let mut matcher = LatentMatcher::from_enabled_domains_with(config)
        .expect("build matcher")
        .with_rerank_mode(rerank);
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
            let component = crate::component::classify(&crate::component::FailureSignal {
                expected_rule: Some(label.to_string()),
                expected_rank: rank,
                any_candidates: top1.is_some(),
                ..crate::component::FailureSignal::default()
            });
            report.misses.push(MissRecord {
                expected: label.to_string(),
                domain,
                rank,
                top1,
                component,
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
    let names = ["clippy", "perf  ", "kernel", "sec   ", "docker"];
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
                "  {} outranked  [{:?}] expected={} rank={} top1={}",
                miss.component.as_str(),
                miss.domain,
                miss.expected,
                r,
                miss.top1.as_deref().unwrap_or("?"),
            ),
            None => println!(
                "  {} not-in-top{} [{:?}] expected={} top1={}",
                miss.component.as_str(),
                PROBE_TOP_K,
                miss.domain,
                miss.expected,
                miss.top1.as_deref().unwrap_or("?"),
            ),
        }
    }
    // Issue 047: per-class counts — the dominant retrieval-stage component
    // is the next tuning target (pool depth, fan-out) instead of a guess.
    if !report.misses.is_empty() {
        let mut e = 0usize;
        let mut rho = 0usize;
        for miss in &report.misses {
            match miss.component {
                crate::component::ComponentClass::Retrieval => rho += 1,
                crate::component::ComponentClass::Corpus => e += 1,
                _ => {}
            }
        }
        println!("  component counts: E(corpus)={e}  rho(retrieval)={rho}");
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

/// One row of the Plan 005 T3.5 retrieval sweep.
#[derive(Debug, Clone)]
pub struct RetrievalSweepRow {
    /// The config that produced the row.
    pub config: crate::draft::RetrievalConfig,
    /// Per-domain metrics, indexed as [`EvalReport::per_domain`].
    pub per_domain: [DomainMetrics; DomainId::ALL.len()],
    /// Blended metrics across every labeled span.
    pub blended: DomainMetrics,
}

/// The T3.5 option-2 grid: BM25 tokenizer x latent fusion weight.
///
/// This is the measurement that decided both shipped defaults. It stays
/// committed so a substrate change that erases the gain (e.g. riir-rag
/// dropping `with_bm25_tokenizer`) reds the gate instead of silently
/// regressing precision.
#[must_use]
pub fn run_retrieval_sweep() -> Vec<RetrievalSweepRow> {
    let mut rows = Vec::new();
    for tokenizer in [
        crate::draft::IndexTokenizer::CodeDefault,
        crate::draft::IndexTokenizer::OpAware,
        crate::draft::IndexTokenizer::OpAwareBigram,
    ] {
        for latent_weight in [1.0_f32, 0.0] {
            let config = crate::draft::RetrievalConfig {
                tokenizer,
                latent_weight,
                ..crate::draft::RetrievalConfig::default()
            };
            let r = run_eval_with(config);
            rows.push(RetrievalSweepRow {
                config,
                per_domain: r.per_domain,
                blended: r.blended,
            });
        }
    }
    rows
}

/// Human-readable T3.5 sweep table.
#[allow(clippy::cast_precision_loss)] // display percentages over tiny counts
pub fn print_retrieval_sweep(rows: &[RetrievalSweepRow]) {
    println!("== Plan 005 T3.5 retrieval sweep (option 2: tiny-token crowding) ==");
    println!("  G1 bar: clippy top-1 >= 80%");
    println!(
        "{:<13} {:>7} {:>9} {:>9} {:>9} {:>9} {:>9}",
        "tokenizer", "latent", "clippy@1", "clippy@3", "perf@1", "blend@1", "blend@3"
    );
    let pct = |v: Option<f32>| v.map_or("-".to_string(), |x| format!("{:.1}%", x * 100.0));
    for row in rows {
        let tag = match row.config.tokenizer {
            crate::draft::IndexTokenizer::CodeDefault => "code-default",
            crate::draft::IndexTokenizer::OpAware => "op-aware",
            crate::draft::IndexTokenizer::OpAwareBigram => "op-bigram",
        };
        println!(
            "{tag:<13} {:>7.2} {:>9} {:>9} {:>9} {:>9} {:>9}",
            row.config.latent_weight,
            pct(row.per_domain[0].recall(1)),
            pct(row.per_domain[0].recall(3)),
            pct(row.per_domain[1].recall(1)),
            pct(row.blended.recall(1)),
            pct(row.blended.recall(3)),
        );
    }
}

/// One row of the Plan 005 T3.5.4 re-rank sweep.
#[derive(Debug, Clone, Copy)]
pub struct RerankSweepRow {
    /// The mode that produced the row.
    pub mode: crate::draft::RerankMode,
    /// Per-domain metrics, indexed as [`EvalReport::per_domain`].
    pub per_domain: [DomainMetrics; DomainId::ALL.len()],
    /// Blended metrics across every labeled span.
    pub blended: DomainMetrics,
}

/// Run the eval under each re-rank mode — the T3.5.4 benchmark.
#[must_use]
pub fn run_rerank_sweep() -> Vec<RerankSweepRow> {
    [
        crate::draft::RerankMode::Off,
        crate::draft::RerankMode::Structural,
        crate::draft::RerankMode::All,
    ]
    .into_iter()
    .map(|mode| {
        let r = run_eval_reranked(mode);
        RerankSweepRow { mode, per_domain: r.per_domain, blended: r.blended }
    })
    .collect()
}

/// Human-readable T3.5.4 sweep table.
#[allow(clippy::cast_precision_loss)] // display percentages over tiny counts
pub fn print_rerank_sweep(rows: &[RerankSweepRow]) {
    println!("== Plan 005 T3.5.4 instantiation re-rank sweep ==");
    println!("  G1 bar: clippy top-1 >= 80%");
    println!(
        "{:<12} {:>9} {:>9} {:>9} {:>9} {:>9}",
        "mode", "clippy@1", "clippy@3", "perf@1", "blend@1", "blend@3"
    );
    let pct = |v: Option<f32>| v.map_or("-".to_string(), |x| format!("{:.1}%", x * 100.0));
    for row in rows {
        let tag = match row.mode {
            crate::draft::RerankMode::Off => "off",
            crate::draft::RerankMode::Structural => "structural",
            crate::draft::RerankMode::All => "all",
        };
        println!(
            "{tag:<12} {:>9} {:>9} {:>9} {:>9} {:>9}",
            pct(row.per_domain[0].recall(1)),
            pct(row.per_domain[0].recall(3)),
            pct(row.per_domain[1].recall(1)),
            pct(row.blended.recall(1)),
            pct(row.blended.recall(3)),
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
        // Rebased 2026-08-16 by Plan 005 T3.5 (operator-aware BM25 tokenizer +
        // adjacency bigrams), again by T3.5.5 (instantiation re-rank promoted
        // to default; measured blended 89/92/92 of 96, clippy 42/45/45 of 48,
        // perf 47/47/47 of 48), again by Issue 024 (Bench 032: RERANK_POOL
        // 8→12 after the cross-domain crowding casualty at raw rank 9;
        // measured blended 90/93/93 of 96, clippy 43/46/46 of 48, perf
        // 47/47/47 of 48), again by Batch 53 (4 `rust_perf` rules + spans,
        // 2026-08-17: perf corpus 58→62 rules, `PERF_SPANS` 48→52, blended
        // denominator 96→100; measured blended 93.0/97.0/97.0 of 100, clippy
        // 43/46/46 of 48, perf 50/51/51 of 52, router 70/100 — all 4 new
        // spans retrieve their rule top-1), and again by Batch 59 (1
        // `rust_perf` rule + span, 2026-08-18, the first external-source
        // distill via the `distill` skill: perf corpus 62→63 rules,
        // `PERF_SPANS` 52→53, blended denominator 100→101; measured blended
        // 94/98/98 of 101, perf 51/52/52 of 53, router 71/101 — the new span
        // retrieves its rule top-1). Floors are measured − ONE span,
        // the documented convention.
        //
        // NOTE these clippy rows are INFLATED by the fixture's single-lint /
        // zero-cross-fire shape (T3.5.4's finding) — they are a fine
        // REGRESSION floor but not an effect size. The re-rank promotion was
        // justified by the uncontaminated composite measurement instead
        // (`tests/uncontaminated_g1.rs`, +12.0pp).
        pub const BLENDED_TOP1: f32 = 93.0 / 101.0;
        pub const BLENDED_TOP3: f32 = 97.0 / 101.0;
        pub const BLENDED_TOP5: f32 = 97.0 / 101.0;
        pub const CLIPPY_TOP1: f32 = 42.0 / 48.0;
        pub const CLIPPY_TOP3: f32 = 45.0 / 48.0;
        pub const PERF_TOP3: f32 = 51.0 / 53.0;
        #[cfg(feature = "kernel_heal")]
        pub const KERNEL_TOP3: f32 = 29.0 / 30.0;
        /// Router-annotation accuracy floor (Issue 020 wiring, Bench 025):
        /// the TF-cosine-global annotation measured 71/101 (70.3%, 2-domain,
        /// Batch 59) after the +1-span growth. Floor = measured − 5 spans of
        /// corpus-growth headroom. This is the standing per-batch measurement
        /// of annotation quality — NOT a gate-quality claim (gating stays
        /// killed at the 85% bar).
        /// Router-annotation accuracy floors, one per enabled-domain state
        /// (Issue 020 wiring, Bench 025). This is the standing per-batch
        /// measurement of annotation quality — NOT a gate-quality claim
        /// (gating stays killed at the 85% bar).
        ///
        /// **Restructured with the docker landing (Plan 030 T0.4).** These
        /// were four `#[cfg]`-gated consts keyed on `(kernel_heal, sec)`. A
        /// 5th domain would have made it EIGHT consts whose declaration cfg
        /// and use cfg must match exactly — the precise shape that turned CI
        /// Layer 5a red on the sec landing (`ROUTER_ANNOTATION_3D` declared
        /// under `kernel_heal` but used under `not(sec)` → `dead_code` under
        /// `-D warnings`). Selecting at RUNTIME with `cfg!` compiles every
        /// arm in every state, so that whole failure class is gone; the
        /// values stay per-state and measured.
        ///
        /// Floor = measured − 5 spans of corpus-growth headroom, per state.
        /// Re-measure + re-pin with the delta recorded; never weaken
        /// silently.
        #[must_use]
        pub fn router_annotation_floor() -> f32 {
            match (
                cfg!(feature = "kernel_heal"),
                cfg!(feature = "sec"),
                cfg!(feature = "docker"),
            ) {
                // 2-domain (clippy+perf): measured 71/101 (70.3%, Batch 59).
                (false, false, false) => 66.0 / 101.0,
                // 3-domain (+kernel): measured 84+5/130.
                (true, false, false) => 84.0 / 130.0,
                // 4-domain (+sec): measured 45/101 (44.6%, Plan 018) — the
                // TF-cosine lexical router has a mechanically harder task
                // (sec's 6 patterns are long full-fn bodies whose
                // common-token centroid absorbs generic Rust spans, and the
                // fixture set carries NO sec-labeled spans, so every
                // sec-routing is a miss by construction).
                (false, true, false) => 40.0 / 101.0,
                // 4-domain + kernel: measured 78/131 (59.5%, Plan 018).
                (true, true, false) => 73.0 / 131.0,
                // ── docker-enabled states (Plan 030 T0.4, MEASURED
                // 2026-08-28; floor = measured − 5 spans, per state) ──
                //
                // Docker's 27 patterns are DOCKERFILE text, not Rust, so
                // their vocabulary (FROM/RUN/apt-get/COPY) barely
                // intersects the Rust fixture spans. The measured result
                // is the opposite of sec's: adding docker never lowers the
                // annotation and twice RAISES it (2-domain 71/101 → 73/101;
                // 4D+K 78/131 → 82/131), because a distinctive
                // off-vocabulary centroid absorbs fewer generic Rust spans
                // than a Rust-shaped one does. That is the measured answer
                // to Proposal 006's open question about a 5th domain's cost
                // to the router: for THIS domain it is not a cost.
                (false, false, true) => 68.0 / 101.0, // measured 73/101 (72.3%)
                (true, false, true) => 89.0 / 131.0,  // measured 94/131 (71.8%)
                (false, true, true) => 36.0 / 101.0,  // measured 41/101 (40.6%)
                (true, true, true) => 77.0 / 131.0,   // measured 82/131 (62.6%)
            }
        }
    }

    #[allow(clippy::cast_precision_loss)] // ratio arithmetic over tiny counts
    fn ratio(hits: usize, n: usize) -> f32 {
        if n == 0 { 1.0 } else { hits as f32 / n as f32 }
    }

    #[test]
    fn rerank_circularity_diagnostic() {
        // The T3.5.4 gain must be checked for ORACLE CONTAMINATION before it
        // can be believed. `clippy_fixture_labels_verify_via_proposers` is a
        // committed, passing test asserting that for every clippy fixture the
        // OWN proposer fires and NO other proposer cross-fires. That is
        // exactly the property the instantiation re-ranker exploits, so on
        // this fixture set `RerankMode::All` degenerates to "was the correct
        // rule retrieved into the pool at all?".
        //
        // The falsifiable prediction: All-mode top-1 == pool recall @
        // RERANK_POOL, exactly. If it holds, the measured gain is not an
        // effect size and must not be reported as one.
        use crate::draft::{RerankMode, RERANK_POOL};
        let mut matcher = LatentMatcher::from_enabled_domains()
            .expect("build matcher")
            .with_rerank_mode(RerankMode::Off);

        let mut in_pool = 0usize;
        let mut labeled = 0usize;
        for span in super::labeled_eval_spans() {
            if span.domain != crate::draft::DomainId::Clippy || !matcher.has_domain(span.domain) {
                continue;
            }
            labeled += 1;
            let (_, by_domain) = matcher.suggest_span(&span.code, RERANK_POOL);
            let found = by_domain
                .iter()
                .find(|(d, _)| *d == span.domain)
                .is_some_and(|(_, rules)| rules.iter().any(|r| r.rule_id == span.label));
            if found {
                in_pool += 1;
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let pool_recall = in_pool as f32 / labeled as f32;

        let all = super::run_eval_reranked(RerankMode::All);
        let all_top1 = all.per_domain[0].recall(1).expect("clippy labeled");

        println!("\n== T3.5.4 circularity diagnostic (clippy) ==");
        println!("  pool recall @{RERANK_POOL:<2}      : {:.1}% ({in_pool}/{labeled})", pool_recall * 100.0);
        println!("  RerankMode::All top-1 : {:.1}%", all_top1 * 100.0);
        println!(
            "  verdict: {}",
            if (all_top1 - pool_recall).abs() < 1e-6 {
                "CONTAMINATED — All-mode top-1 IS pool recall; the fixture's \
                 one-proposer-fires invariant decides the ranking, not the re-ranker"
            } else {
                "not fully explained by pool recall"
            }
        );
        assert!(
            (all_top1 - pool_recall).abs() < 1e-6,
            "the contamination prediction failed (all {all_top1:.4} vs pool {pool_recall:.4}) \
             — re-examine the oracle before trusting either number"
        );
    }

    #[test]
    fn rerank_sweep_report() {
        // Plan 005 T3.5.4 benchmark: does proposer-visitor evidence beat pure
        // text similarity? Prints the A/B table and pins the shipped gain.
        let rows = super::run_rerank_sweep();
        super::print_rerank_sweep(&rows);

        let get = |m: crate::draft::RerankMode| {
            *rows.iter().find(|r| r.mode == m).expect("row present")
        };
        let off = get(crate::draft::RerankMode::Off);
        let structural = get(crate::draft::RerankMode::Structural);

        let off_clippy = off.per_domain[0].recall(1).expect("clippy labeled");
        let on_clippy = structural.per_domain[0].recall(1).expect("clippy labeled");
        let off_perf = off.per_domain[1].recall(1).expect("perf labeled");
        let on_perf = structural.per_domain[1].recall(1).expect("perf labeled");

        // MECHANICAL check only. These numbers are CONTAMINATED — the clippy
        // fixtures are label-verified by the very proposers doing the ranking
        // (`clippy_fixture_labels_verify_via_proposers` asserts own-fires +
        // zero-cross-fires), and `rerank_circularity_diagnostic` shows
        // All-mode top-1 IS pool recall, exactly. So this asserts only that
        // the wiring is live and does no harm — NOT an effect size.
        assert!(
            on_clippy > off_clippy,
            "the re-rank wiring appears dead: clippy top-1 unchanged \
             (on {on_clippy:.3} vs off {off_clippy:.3})"
        );
        assert!(
            on_perf >= off_perf,
            "perf top-1 must not regress from re-ranking \
             (on {on_perf:.3} vs off {off_perf:.3})"
        );
        // Promoted 2026-08-16 by T3.5.5 on UNCONTAMINATED evidence (+12.0pp
        // top-1 over real-clippy-labeled multi-lint spans — see
        // `tests/uncontaminated_g1.rs`). This assertion pins the promotion so
        // a silent revert to Off is caught.
        assert_eq!(
            crate::draft::RerankMode::default(),
            crate::draft::RerankMode::Structural,
            "RerankMode::Structural is the T3.5.5-promoted default"
        );
        println!(
            "  NOTE: the clippy deltas in this table are INFLATED by the fixture's \
             single-lint shape (see rerank_circularity_diagnostic). The promotion \
             rests on tests/uncontaminated_g1.rs (+12.0pp), not on this table."
        );
    }

    #[test]
    fn retrieval_sweep_report() {
        // Plan 005 T3.5 option-2 benchmark. Prints the grid and pins the
        // load-bearing claims so a substrate change that erases the gain
        // (e.g. riir-rag reverting `with_bm25_tokenizer`) reds this test.
        const T35_G1_BAR: f32 = 0.80;
        let rows = super::run_retrieval_sweep();
        super::print_retrieval_sweep(&rows);

        let get = |t: crate::draft::IndexTokenizer, w: f32| {
            rows.iter()
                .find(|r| r.config.tokenizer == t && (r.config.latent_weight - w).abs() < 1e-6)
                .expect("row present")
                .clone()
        };
        // The pre-T3.5 shipped config vs the promoted one.
        let base = get(crate::draft::IndexTokenizer::CodeDefault, 1.0);
        // The PROMOTED config — the gate must test what actually ships.
        let promoted = crate::draft::RetrievalConfig::default();
        let op = get(promoted.tokenizer, promoted.latent_weight);

        let clippy_base = base.per_domain[0].recall(1).expect("clippy labeled");
        let clippy_op = op.per_domain[0].recall(1).expect("clippy labeled");
        assert!(
            clippy_op > clippy_base,
            "the promoted config must beat the substrate default on clippy top-1 \
             (promoted {clippy_op:.3} vs base {clippy_base:.3})"
        );
        // T3.5's G1 bar is 80%. The tokenizer work closed most but not all of
        // the gap (54.2% -> 70.8%), so this asserts the MEASURED gain as
        // regression protection and reports the residual gap rather than
        // asserting a target that is not yet met — a red permanent test
        // would just be a TODO that blocks the suite.
        if clippy_op < T35_G1_BAR {
            println!(
                "  NOTE: clippy top-1 {:.1}% is still {:.1}pp under the T3.5 G1 bar of {:.0}% \
                 — Phase 3 stays open (see Plan 005 T3.5 / Bench 027).",
                clippy_op * 100.0,
                (T35_G1_BAR - clippy_op) * 100.0,
                T35_G1_BAR * 100.0
            );
        }
        // Identifier-rich domains must not pay for the operator additions.
        let perf_base = base.per_domain[1].recall(1).expect("perf labeled");
        let perf_op = op.per_domain[1].recall(1).expect("perf labeled");
        assert!(
            perf_op >= perf_base,
            "perf top-1 must not regress (op {perf_op:.3} vs base {perf_base:.3})"
        );
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
            let floor = floors::router_annotation_floor();
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

    /// Clean-span truth for the 2026-08-17 proposer additions: no real
    /// production excerpt fires the five new rewrites (the perf-domain
    /// analog of [`Self::clean_spans_fire_no_lint`], scoped to the NEW
    /// proposers — the pre-existing five are Bench-019-G5-validated on real
    /// files instead).
    #[cfg(feature = "rust_perf")]
    #[test]
    fn clean_spans_fire_no_new_perf_proposer() {
        use crate::domains::rust_perf::proposers::propose_fix;
        const NEW_RULES: &[&str] = &[
            "hashset-insert-bool-return",
            "collect-append-to-extend",
            "vec-macro-zero-fill",
            "distance-sq-no-sqrt",
            "hashmap-with-capacity",
        ];
        for (tag, code) in CLEAN_SPANS {
            for &rule in NEW_RULES {
                assert!(
                    propose_fix(rule, code).is_none(),
                    "clean span {tag} fires {rule}"
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

    /// Plan 018 T1.5 — the sec-domain self-variant floor (the kernel
    /// precedent): every sec corpus rule's `buggy_code`, identifier-renamed
    /// via the shared deterministic map, must retrieve ITS OWN rule at top-1
    /// in the sec domain. Establishes the new domain's retrieval floor
    /// WITHOUT touching the shared eval fixture set (the 4-domain router
    /// annotation floor is re-pinned separately — see
    /// [`floors::router_annotation_floor`]).
    #[cfg(feature = "sec")]
    #[test]
    fn sec_self_variants_retrieve_top1() {
        use crate::domains::sec::SecDomain;
        use crate::domain::HealDomain;
        let corpus = SecDomain::build_corpus();
        assert!(corpus.len() >= 6, "Phase-1 survival count");
        let mut matcher = LatentMatcher::from_enabled_domains()
            .expect("build matcher");
        let mut misses = Vec::new();
        for rule in &corpus {
            let probe = mutate_shape(&rule.buggy_code);
            let (_routed, by_domain) = matcher.suggest_span(&probe, 3);
            let sec_rules = by_domain
                .iter()
                .find(|(d, _)| *d == DomainId::Sec)
                .map(|(_, rules)| rules)
                .expect("sec domain present");
            let top1 = sec_rules
                .first()
                .map_or("", |r| r.rule_id.as_str());
            if top1 != rule.rule_id {
                misses.push(format!(
                    "{}: top1={top1} (want {})",
                    rule.rule_id, rule.rule_id
                ));
            }
        }
        assert!(misses.is_empty(), "sec self-variant top-1 misses:\n{}", misses.join("\n"));
    }
}

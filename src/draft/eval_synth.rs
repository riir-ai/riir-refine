//! Issue 186 — structure-grounded synthetic retrieval-eval fixtures.
//!
//! ## The STAIR recipe, transferred
//!
//! Oracle-labeled eval data is the healer's scarcest artifact (Issue 030's
//! relevance gate, the `clippy_oracle` live-gap signals). Research 311's
//! reading of STAIR (arXiv:2609.03874) supplies the recipe that scales it:
//! **derive gold from the artifact's own structure, generate only the
//! questions.** Here the artifact is the corpus itself — each rule's
//! `buggy_code` IS the structural truth — so a span derived from rule R's
//! own material carries gold = R with **zero teacher opinion in any label**
//! and zero GPU in its generation.
//!
//! ## Tier 1 — deterministic template mutations (this module)
//!
//! The questions are surface mutations of the corpus span, in three classes
//! (the issue's named set; nothing invented):
//!
//! | mutation | what it varies | what it measures |
//! |---|---|---|
//! | `ident-rename` | every non-keyword identifier gains an `x` prefix | naming robustness — the embedder must not key on identifiers |
//! | `comment-strip` | `//`-family comments removed (string-aware) | comment-noise robustness |
//! | `reindent` | every non-empty line gains 4 leading spaces | layout robustness |
//!
//! A mutation that leaves the text byte-identical is **skipped** (`None`):
//! the corpus text is already an index chunk, so an identity query would
//! measure exact-match lookup, not retrieval. `reindent` never fires
//! identity on non-empty code; the other two fire per-span. The
//! `ident-rename` transform is the `mutate_shape` kinship made general —
//! the kernel probe's fixed word map ([`super::retrieval_eval::mutate_shape`])
//! renames a hand-picked vocabulary; this renames EVERYTHING, which is what
//! makes the variant a naming-blind query rather than a paraphrase.
//!
//! Tier 2 (teacher paraphrase under the echo-gate + leak-scan discipline)
//! is deliberately NOT here — it is a separate, owner-gated lane.
//!
//! ## Consumers
//!
//! - [`clippy_synth_population`] feeds [`super::withheld_pair_ood`] — the
//!   corpus alone gives each rule ONE binding, so the withheld-pair
//!   eligibility floor (`|cell| >= 2`) is mostly unmeetable on fixture
//!   populations; the variants give rules 2–4 bindings and the composition
//!   axis its power. Whole RULES are withheld from the index exactly as the
//!   protocol already does — the fixtures change the population, never the
//!   arms.
//! - [`run_synth_floor`] measures the mutated queries against the standard
//!   matcher — the robustness floor the hand-authored fixture sets bound
//!   from the other side. It is a SEPARATE set from
//!   [`super::retrieval_eval::labeled_eval_spans`] on purpose: merging it
//!   would move every pinned eval count; the family stands alone until a
//!   gate wants it merged.
//!
//! Modelless end to end: deterministic string transforms + the shipped
//! retrieval stack. No weights, no training, no GPU.

use super::domain_router::DomainId;
use super::retrieval_eval::{canonical_rank, DomainMetrics};
use super::withheld_pair_ood::{Binding, RUST_KEYWORDS};

/// How deep the per-domain ranked list is probed — the house probe depth
/// ([`super::retrieval_eval`] uses the same 8).
const PROBE_TOP_K: usize = 8;

// ── the mutations ───────────────────────────────────────────────────────

/// One deterministic tier-1 mutation. Fixed order = [`SYNTH_MUTATIONS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynthMutation {
    /// Every non-keyword identifier gains an `x` prefix.
    IdentRename,
    /// `//`-family comments removed, string-aware.
    CommentStrip,
    /// Every non-empty line gains 4 leading spaces.
    Reindent,
}

/// The mutation set in its fixed (report-row) order.
pub const SYNTH_MUTATIONS: [SynthMutation; 3] = [
    SynthMutation::IdentRename,
    SynthMutation::CommentStrip,
    SynthMutation::Reindent,
];

impl SynthMutation {
    /// Stable label — the report key.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::IdentRename => "ident-rename",
            Self::CommentStrip => "comment-strip",
            Self::Reindent => "reindent",
        }
    }

    /// Apply the mutation. `None` = identity (nothing to measure — see the
    /// module docs); the caller skips the span.
    #[must_use]
    pub fn apply(self, code: &str) -> Option<String> {
        let out = match self {
            Self::IdentRename => rename_identifiers(code),
            Self::CommentStrip => strip_comments(code),
            Self::Reindent => reindent(code),
        };
        if out == code {
            None
        } else {
            Some(out)
        }
    }
}

/// Every non-keyword identifier gains an `x` prefix — word-boundary-exact
/// (the same char-walk shape as [`super::retrieval_eval::mutate_shape`]).
///
/// Keywords, lifetimes (`'a` — the quote breaks the token), and raw-ident
/// hashes are untouched by construction; a rename inside a string literal
/// is POSSIBLE here (a name appearing in a format string also renames),
/// which is correct for a retrieval query — the mutation is a surface
/// transform, not a semantics-preserving rewrite.
fn rename_identifiers(code: &str) -> String {
    let mut out = String::with_capacity(code.len() + 16);
    let mut token = String::new();
    let flush = |token: &mut String, out: &mut String| {
        if !token.is_empty() {
            let renamed = if token.chars().all(|c| c.is_ascii_digit())
                || RUST_KEYWORDS.contains(&token.as_str())
            {
                token.as_str()
            } else {
                // `x` + name: word-boundary-safe (never merges into a
                // preceding token — the boundary char precedes it).
                out.reserve(1);
                token.insert(0, 'x');
                token.as_str()
            };
            out.push_str(renamed);
            token.clear();
        }
    };
    for ch in code.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else {
            flush(&mut token, &mut out);
            out.push(ch);
        }
    }
    flush(&mut token, &mut out);
    out
}

/// Remove every `//`-to-end-of-line comment outside string/char literals.
/// Line continuations inside strings (`\` at line end) are honored; block
/// comments are left alone (rare in the corpus; a v2 concern at most).
fn strip_comments(code: &str) -> String {
    let b = code.as_bytes();
    let mut out = String::with_capacity(code.len());
    let mut i = 0usize;
    // literal state: which quote is open, if any
    let mut open: Option<u8> = None;
    while i < b.len() {
        let c = b[i];
        if let Some(q) = open {
            out.push(c as char);
            if c == b'\\' {
                // escape: copy the next byte verbatim (multibyte UTF-8
                // continuation bytes pass through the `as char` losslessly
                // only for ASCII — handle non-ASCII by copying the byte)
                if i + 1 < b.len() {
                    i += 1;
                    out.push(b[i] as char);
                }
            } else if c == q {
                open = None;
            }
            i += 1;
            continue;
        }
        if c == b'"' || c == b'\'' {
            open = Some(c);
            out.push(c as char);
            i += 1;
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            // comment: skip to end of line (the newline itself is kept)
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // non-ASCII passes through byte-wise
        out.push(c as char);
        i += 1;
    }
    out
}

/// Every non-empty line gains 4 leading spaces.
fn reindent(code: &str) -> String {
    let mut out = String::with_capacity(code.len() + 4 * code.lines().count());
    for line in code.lines() {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// All tier-1 variants of one span, in [`SYNTH_MUTATIONS`] order,
/// identity-skipped. `(mutation label, mutated text)`.
#[must_use]
pub fn synth_variants(code: &str) -> Vec<(&'static str, String)> {
    SYNTH_MUTATIONS
        .iter()
        .filter_map(|m| m.apply(code).map(|out| (m.label(), out)))
        .collect()
}

// ── the structure-grounded span sets ────────────────────────────────────

/// One synth eval span: the owning rule (gold), the mutated text, and which
/// mutation produced it.
#[derive(Debug, Clone)]
pub struct SynthSpan {
    pub rule: String,
    pub code: String,
    pub mutation: &'static str,
}

/// The structure-grounded span set over `(rule, code)` pairs — every
/// non-empty span's tier-1 variants. Provenance is structural by
/// construction: the gold is the rule whose own material the span mutates.
#[must_use]
pub fn synth_spans<'a>(entries: impl Iterator<Item = (&'a str, &'a str)>) -> Vec<SynthSpan> {
    let mut out = Vec::new();
    for (rule, code) in entries {
        if code.trim().is_empty() {
            continue;
        }
        for (label, mutated) in synth_variants(code) {
            out.push(SynthSpan {
                rule: rule.to_string(),
                code: mutated,
                mutation: label,
            });
        }
    }
    out.shrink_to_fit();
    out
}

/// The clippy domain's synth span set — the default-on domain, wired so the
/// standing lanes exercise the family without any feature above default.
#[cfg(feature = "clippy_lints")]
#[must_use]
pub fn clippy_synth_spans() -> Vec<SynthSpan> {
    use crate::domain::{HealDomain, HealEntry};
    let corpus = crate::domains::clippy_lints::ClippyLintDomain::build_corpus();
    synth_spans(corpus.iter().map(|e| (e.rule_id(), e.buggy_code())))
}

/// The `rust_perf` domain's synth span set (the other default-on domain).
#[cfg(feature = "rust_perf")]
#[must_use]
pub fn perf_synth_spans() -> Vec<SynthSpan> {
    use crate::domain::{HealDomain, HealEntry};
    let corpus = crate::domains::rust_perf::RustPerfDomain::build_corpus();
    synth_spans(corpus.iter().map(|e| (e.rule_id(), e.buggy_code())))
}

/// The withheld-pair population over the clippy corpus + its synth variants:
/// the corpus binding (source `corpus`) plus every variant (source `synth`).
///
/// Feeding [`super::withheld_pair_ood::run_withheld_pair`] with this gives
/// the protocol's eligibility floor (`|cell| >= 2`) real power: the corpus
/// alone puts one binding per rule, the variants put each rule's cell under
/// its own shape class 2–4 deep. Whole rules are withheld exactly as the
/// protocol always has — the fixtures change the population, never the arms.
#[cfg(feature = "clippy_lints")]
#[must_use]
pub fn clippy_synth_population() -> Vec<Binding> {
    use crate::domain::{HealDomain, HealEntry};
    let corpus = crate::domains::clippy_lints::ClippyLintDomain::build_corpus();
    let mut out = Vec::with_capacity(corpus.len() * 4);
    for e in &corpus {
        let rule = e.rule_id();
        let code = e.buggy_code();
        if code.trim().is_empty() {
            continue;
        }
        out.push(Binding::clippy(rule, code, "corpus"));
        for (_label, mutated) in synth_variants(code) {
            out.push(Binding {
                rule: rule.to_string(),
                shape: super::withheld_pair_ood::classify_shape(&mutated).label(),
                text: mutated,
                source: "synth",
            });
        }
    }
    out.shrink_to_fit();
    out
}

// ── the floor ───────────────────────────────────────────────────────────

/// The synth robustness floor: every span answered by its own domain's
/// engine under the shipped posture (default config, default re-rank), the
/// same read [`super::retrieval_eval::run_eval_on`] gives the hand-authored
/// sets. Blended + per-mutation recall.
///
/// # Panics
///
/// Panics if the matcher cannot build (no domain feature enabled).
#[must_use]
pub fn run_synth_floor() -> SynthReport {
    let mut matcher = super::latent_matcher::LatentMatcher::from_enabled_domains()
        .expect("build matcher")
        .with_rerank_mode(crate::draft::RerankMode::default());
    run_synth_floor_on(&mut matcher)
}

/// [`run_synth_floor`] over a caller-built matcher (the A/B seam, the
/// [`super::retrieval_eval::run_eval_on`] convention).
///
/// # Panics
///
/// Panics if the domain spans set is empty AND the matcher has no domains
/// (both default-on domains compiled out — the thin-feature posture).
#[must_use]
pub fn run_synth_floor_on(matcher: &mut super::latent_matcher::LatentMatcher) -> SynthReport {
    let mut spans: Vec<(DomainId, SynthSpan)> = Vec::new();
    #[cfg(feature = "clippy_lints")]
    spans.extend(
        clippy_synth_spans()
            .into_iter()
            .map(|s| (DomainId::Clippy, s)),
    );
    #[cfg(feature = "rust_perf")]
    spans.extend(perf_synth_spans().into_iter().map(|s| (DomainId::Perf, s)));

    let mut report = SynthReport::default();
    for (domain, span) in &spans {
        if !matcher.has_domain(*domain) {
            report.skipped += 1;
            continue;
        }
        let (_, by_domain) = matcher.suggest_span(&span.code, PROBE_TOP_K);
        let ranked = by_domain
            .iter()
            .find(|(d, _)| *d == *domain)
            .map_or(Vec::new(), |(_, rules)| canonical_rank(rules));
        let rank = ranked.iter().position(|r| r.rule_id == span.rule);

        report.blended.labeled += 1;
        if rank.is_some_and(|r| r < 1) {
            report.blended.hits_at_1 += 1;
        }
        if rank.is_some_and(|r| r < 3) {
            report.blended.hits_at_3 += 1;
        }
        if rank.is_some_and(|r| r < 5) {
            report.blended.hits_at_5 += 1;
        }
        let row = report.by_mutation.entry(span.mutation).or_default();
        row.labeled += 1;
        if rank.is_some_and(|r| r < 1) {
            row.hits_at_1 += 1;
        }
        if rank.is_some_and(|r| r < 3) {
            row.hits_at_3 += 1;
        }
        if rank.is_some_and(|r| r < 5) {
            row.hits_at_5 += 1;
        }
    }
    report.n_spans = report.blended.labeled;
    report
}

/// The synth floor's report: blended recall + one row per mutation.
#[derive(Debug, Default, Clone)]
pub struct SynthReport {
    /// Spans actually scored (matched-domain present).
    pub n_spans: usize,
    /// Spans whose domain feature is off.
    pub skipped: usize,
    /// Blended recall over all scored spans.
    pub blended: DomainMetrics,
    /// Recall per mutation, keyed by [`SynthMutation::label`], in
    /// [`SYNTH_MUTATIONS`] order when printed.
    pub by_mutation: std::collections::BTreeMap<&'static str, DomainMetrics>,
}

/// The `--nocapture` table.
pub fn print_synth_report(r: &SynthReport) {
    println!("\n== Issue 186 structure-grounded synth floor ==");
    println!(
        "  scored {} spans ({} skipped, domain off)",
        r.n_spans, r.skipped
    );
    println!(
        "  {:<34} {:>6} {:>7} {:>7} {:>7}",
        "mutation", "n", "top-1", "top-3", "top-5"
    );
    println!(
        "  {:<34} {:>6} {:>6.1}% {:>6.1}% {:>6.1}%",
        "BLENDED",
        r.blended.labeled,
        r.blended.recall(1).unwrap_or(0.0) * 100.0,
        r.blended.recall(3).unwrap_or(0.0) * 100.0,
        r.blended.recall(5).unwrap_or(0.0) * 100.0,
    );
    for m in SYNTH_MUTATIONS {
        let Some(row) = r.by_mutation.get(m.label()) else {
            continue;
        };
        println!(
            "  {:<34} {:>6} {:>6.1}% {:>6.1}% {:>6.1}%",
            m.label(),
            row.labeled,
            row.recall(1).unwrap_or(0.0) * 100.0,
            row.recall(3).unwrap_or(0.0) * 100.0,
            row.recall(5).unwrap_or(0.0) * 100.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieval_synth_mutations_are_deterministic() {
        let code = "pub fn alpha(items: &[u32]) -> u32 {\n    // sum the items\n    items.iter().sum()\n}\n";
        for m in SYNTH_MUTATIONS {
            let once = m.apply(code);
            let twice = m.apply(code);
            assert_eq!(once, twice, "{m:?} must be deterministic");
        }
    }

    #[test]
    fn retrieval_synth_rename_erases_names_keeps_structure() {
        let code = "fn alpha(beta: u8) -> u8 { beta.wrapping_add(1) }";
        let renamed = SynthMutation::IdentRename.apply(code).expect("renames");
        assert!(
            renamed.contains("xalpha"),
            "identifiers prefixed: {renamed}"
        );
        assert!(!renamed.contains("fn xfn"), "keywords untouched: {renamed}");
        // The atomic-null invariant: the renamed text normalizes to the SAME
        // token stream (identifiers collapse to `I` on both sides).
        let a = super::super::withheld_pair_ood::normalized_tokens(code);
        let b = super::super::withheld_pair_ood::normalized_tokens(&renamed);
        assert_eq!(a, b, "rename must be invisible to the atomic key");
        // Numbers are not identifiers.
        assert!(renamed.contains('1'), "literals untouched: {renamed}");
    }

    #[test]
    fn retrieval_synth_comment_strip_is_string_aware() {
        let code = "fn f() -> &str {\n    // the greeting\n    let s = \"http://not-a-comment\";\n    s\n}\n";
        let stripped = SynthMutation::CommentStrip.apply(code).expect("strips");
        assert!(!stripped.contains("// the greeting"), "comment gone");
        assert!(
            stripped.contains("http://not-a-comment"),
            "string content survives: {stripped}"
        );
        // No comments → identity → None (skipped, never measured).
        let clean = "fn f() -> u8 { 1 }";
        assert_eq!(SynthMutation::CommentStrip.apply(clean), None);
    }

    #[test]
    fn retrieval_synth_reindent_grows_every_line() {
        let code = "fn f() -> u8 {\n    1\n}\n";
        let out = SynthMutation::Reindent.apply(code).expect("grows");
        for line in out.lines() {
            assert!(line.starts_with("    "), "every line indented: {line:?}");
        }
        assert_ne!(out, code);
    }

    #[test]
    fn retrieval_synth_gold_is_structural_membership() {
        // The STAIR property: every synth span's gold rule is the rule whose
        // OWN material the span mutates — asserted over the live clippy
        // corpus (default-on), the label cannot drift from the structure.
        use crate::domain::{HealDomain, HealEntry};

        let spans = clippy_synth_spans();
        assert!(!spans.is_empty(), "the clippy corpus yields variants");
        let corpus = crate::domains::clippy_lints::ClippyLintDomain::build_corpus();
        let rules: std::collections::HashSet<&str> = corpus
            .iter()
            .map(crate::domain::HealEntry::rule_id)
            .collect();
        for s in &spans {
            assert!(
                rules.contains(s.rule.as_str()),
                "gold {} is a corpus rule (structural membership)",
                s.rule
            );
        }
        // And every mutated text differs from its own source (no identity
        // query ever enters the set).
        for s in &spans {
            let src = corpus
                .iter()
                .find(|e| e.rule_id() == s.rule)
                .map(crate::domain::HealEntry::buggy_code)
                .expect("gold rule present");
            assert_ne!(s.code, src, "identity spans are skipped");
        }
    }

    #[test]
    fn retrieval_synth_population_grows_cells() {
        // The withheld-pair feeding property: corpus + variants puts every
        // rule's binding count ABOVE the corpus-only population's one-per-
        // rule floor for at least the rules that yielded variants — the
        // eligibility power the module docs claim.
        let pop = clippy_synth_population();
        let corpus_only = pop.iter().filter(|b| b.source == "corpus").count();
        let synth = pop.iter().filter(|b| b.source == "synth").count();
        assert!(corpus_only > 0);
        assert!(
            synth >= corpus_only,
            ">=1 variant per corpus entry (reindent always fires)"
        );
        // No variant collides with its corpus text (index dedup safety).
        let texts: std::collections::HashSet<&str> = pop.iter().map(|b| b.text.as_str()).collect();
        assert_eq!(texts.len(), pop.len(), "all texts distinct (dedup-safe)");
    }

    #[test]
    #[ignore = "priced: the full withheld-pair protocol over the synth population (an index build per eligible cell) — run at landings: --ignored --nocapture"]
    fn retrieval_synth_withheld_pair_protocol_reads() {
        // The feeding property, MEASURED: the synth population gives the
        // protocol eligible cells (the corpus-only population gives almost
        // none — one binding per rule cannot pass |cell| >= 2). NOROLE must
        // read 0.0 (the dead-instrument canary) or the harness is broken.
        let pop = clippy_synth_population();
        let report = super::super::withheld_pair_ood::run_withheld_pair(
            pop,
            None,
            crate::draft::RerankMode::default(),
            "synth-population",
        );
        assert!(report.eligible_cells > 0, "the synth family feeds the axis");
        assert!(
            report.norole_topk(1) == 0.0,
            "NOROLE canary must be 0.0 by construction"
        );
        super::super::withheld_pair_ood::print_report(&report);
    }

    #[test]
    #[ignore = "priced: one full two-domain matcher build ≈ +90s wall — run at corpus landings / re-pins: --ignored --nocapture"]
    fn retrieval_synth_floor_measures_and_holds_a_floor() {
        // The always-on half is the DETERMINISM + structural pins above; the
        // matcher-based floor is #[ignore]d below (one full two-domain
        // matcher build ≈ +90s wall on the shared box — the freeze-table
        // convention). Run at corpus landings + before re-pinning:
        //   cargo test --lib --features latent_retrieval retrieval_synth -- --ignored --nocapture
        // The floor is a FLOOR (the set grows with the corpus; exact pins
        // would churn every batch — the kernel_fixture_spans convention).
        // Re-pin only WITH a recorded re-measure, never silently.
        // Landing read 2026-10-28: 1005 spans, blended 97.7/99.9/100.0
        // (ident-rename 96.9 / comment-strip 97.9 / reindent 98.4 top-1) —
        // pinned at 0.50 top-1, a collapse floor, not a tuned bar.
        let report = run_synth_floor();
        assert!(report.n_spans > 0, "the synth family is live");
        let top1 = report.blended.recall(1).unwrap_or(0.0);
        assert!(
            top1 >= 0.50,
            "synth top-1 {top1:.3} fell below the 0.50 floor — re-measure + re-pin with the delta recorded"
        );
        // Every mutation row that has spans is never vacuously perfect
        // against an EMPTY set (recall() guards n=0).
        for m in SYNTH_MUTATIONS {
            if let Some(row) = report.by_mutation.get(m.label()) {
                assert_eq!(row.labeled > 0, row.recall(1).is_some());
            }
        }
        print_synth_report(&report);
    }
}

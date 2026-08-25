//! Issue 040 — the fixer re-gate POC harness (partial-credit metrics +
//! modelless pre-pass + CF Workers AI).
//!
//! # Why this exists
//!
//! The Proposal 005 fixer blocker cites Plan 336 G1 = 0/60 twice. Three facts
//! weaken that number: it was **tied with the frozen-backbone control**, the
//! metric is brutal minimal-diff **exact match** (nobody has measured
//! partial-credit quality: compiles? oracle-verified? fewer findings?), and
//! the eval corpus was rebuilt twice mid-campaign. The falsifiable question
//! ([`crate` issue 040]):
//!
//! > Does **verify-filtered acceptance** beat `decline` (no fix) on the
//! > reachable-miss set, under partial-credit metrics, on any arm?
//!
//! # Arms
//!
//! | arm | model | pre-pass |
//! |---|---|---|
//! | A | CF `@cf/meta/llama-3.3-70b-instruct-fp8-fast` (REST) | none ([`prompt_raw`]) |
//! | B | same | [`prompt_prepass`]: span + modelless candidate + corpus few-shots |
//! | C | local Bonsai via [`crate::l4_daemon::DaemonL4`] | same pre-pass |
//!
//! # Substrate reuse map (the issue's "~80% shipped" claim, made concrete)
//!
//! - reachable-miss set: the 9 out-of-corpus fixtures from
//!   `tests/t51_l4_vs_decline.rs` (Bench 037/039), copied deliberately per
//!   that file's own precedent, premise re-asserted by
//!   [`assert_reachable_premise`].
//! - miss classification + modelless candidate: [`crate::self_evolve::HealPipeline`]
//!   seeded from the real corpus (the t51 `seeded_pipeline` shape).
//! - few-shots: [`crate::draft::LatentMatcher`] (the `--suggest` ranking) +
//!   its `rule_book` for buggy/fixed pattern pairs.
//! - fragment parse gate: [`crate::pruner::RustAnalyzerPruner`] (L2, the
//!   seam's actual invariant-3 gate).
//! - oracle: [`crate::ClippyVerifier`] with the t51 `control_fires` scaffold
//!   discipline (a case is scoreable only if the BUGGY input trips a target
//!   lint in the same scaffold).
//! - never-wrong keep rule: strictly fewer clippy findings, plus the two
//!   Bench 039 false-WIN vetoes — contract preservation (destructive edits)
//!   and [`crate::behavioural_equivalence`] (silent logic changes).
//! - edit application: the v2edit "only the corrected lines that differ"
//!   contract — [`apply_edit`] tries 4 deterministic strategies
//!   (verbatim → fn-span substitution (riir-train `rlvr_reward::substitute`
//!   semantics, copied per the t51 copy-don't-share precedent) → LCS merge
//!   depth-bounded → unbounded), first-parses-wins, strategy reported.
//! - CF transport: `ureq` behind this feature; token comes from
//!   `CF_API_TOKEN` and is NEVER logged or included in any error string.

use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use crate::ClippyVerifier;
use crate::behavioural_equivalence::{self, Verdict};
use crate::draft::{DomainId, LatentMatcher};
use crate::domain::HealEntry;
use crate::l4_daemon::TRAINED_INSTRUCTION;
use crate::pruner::RustAnalyzerPruner;
use crate::self_evolve::HealPipeline;

/// The clippy seed corpus type (the [`crate::build_seed_corpus`] return).
pub type ClippyCorpus = crate::Corpus<crate::ClippyLintDomain>;

// ── Reachable-miss set (copied from tests/t51_l4_vs_decline.rs) ────────────

/// The reachable set: real clippy lints verified ABSENT from the seed corpus.
///
/// Copied deliberately from `tests/t51_l4_vs_decline.rs` (which itself copied
/// from `benches/bench_003_modelless_miss_rate.rs`) rather than shared — the
/// premise is re-asserted by [`assert_reachable_premise`] so corpus growth
/// fails loudly instead of silently rewriting this fixture.
pub const REACHABLE: &[(&str, &str)] = &[
    (
        "or_fun_call",
        "pub fn f(opt: Option<u32>) -> u32 { opt.unwrap_or(expensive()) }\npub fn expensive() -> u32 { 42 }",
    ),
    ("large_enum_variant", "pub enum E { A(u8), B([u8; 4096]) }"),
    (
        "too_many_arguments",
        "pub fn f(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) { let _ = (a,b,c,d,e,f,g,h); }",
    ),
    (
        "redundant_clone",
        "pub fn f() { let s = String::from(\"hi\"); let s2 = s.clone(); drop(s2); }",
    ),
    (
        "vec_init_then_push",
        "pub fn f() -> Vec<i32> { let mut v = Vec::new(); v.push(1); v.push(2); v }",
    ),
    (
        "manual_strip",
        "pub fn f(s: &str) -> Option<&str> { if s.starts_with(\"ab\") { Some(&s[2..]) } else { None } }",
    ),
    (
        "box_collection",
        "pub struct S { v: Box<Vec<u8>> }\npub fn mk() -> S { S { v: Box::new(Vec::new()) } }",
    ),
    (
        "option_if_let_else",
        "pub fn f(o: Option<u32>) -> u32 { if let Some(x) = o { x + 1 } else { 0 } }",
    ),
    ("ptr_arg", "pub fn f(v: &Vec<u8>) -> usize { v.len() }"),
];

/// Assert the premise of the whole benchmark: these lints really are absent
/// from the seed corpus (copied from t51, same rationale).
///
/// # Panics
///
/// Panics if any reachable lint has grown a corpus entry — the fixture must
/// be re-picked, not silently re-scored against a different baseline.
pub fn assert_reachable_premise(corpus: &ClippyCorpus) {
    for (lint, _) in REACHABLE {
        let n = corpus
            .entries
            .iter()
            .filter(|e| e.rule_id() == *lint)
            .count();
        assert_eq!(
            n, 0,
            "`{lint}` was expected OUT of corpus but has {n} candidate(s) — \
             the corpus grew; this fixture must be re-picked"
        );
    }
}

// ── Samples ────────────────────────────────────────────────────────────────

/// One eval sample.
#[derive(Debug, Clone)]
pub struct Sample {
    /// Stable id (`reachable:ptr_arg` / `v2edit:c00:007`).
    pub id: String,
    /// Target lints (>=1; v2edit rows may be multi-lint).
    pub lints: Vec<String>,
    /// The buggy code (a whole module for v2edit rows, an item set for
    /// reachable fixtures).
    pub module: String,
    /// The edit-style expected response (v2edit rows only) — EM compat column.
    pub expected: Option<String>,
    /// Where the sample came from (drives the merge strategy order).
    pub source: SampleSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleSource {
    /// A single-item (or two-item) fixture from the reachable set — the
    /// model's output may legitimately be a verbatim full replacement.
    Reachable,
    /// A v2edit chunk row — the response contract is edit-lines, so verbatim
    /// whole-module replacement is NOT admissible (it would drop the other
    /// fns and fake a findings reduction).
    V2edit,
}

/// Strip the shared instruction prefix from a v2edit prompt row.
///
/// Same semantics as riir-train's `rlvr_reward::prompt_module` (marker
/// `"no explanation:\n\n"`), reimplemented locally because riir-train is not
/// an allowed dep for this repo (BOUNDARY.md) — the t51 copy-don't-share
/// precedent.
#[must_use]
pub fn prompt_module(prompt: &str) -> &str {
    const MARKER: &str = "no explanation:\n\n";
    match prompt.find(MARKER) {
        Some(at) => &prompt[at + MARKER.len()..],
        None => prompt,
    }
}

/// Extract the content of the first markdown code fence, if fenced.
///
/// Applied SYMMETRICALLY to generated + expected text so EM measures code
/// content, not markdown-wrapper compliance (same contract as the riir-train
/// eval driver; the base model habitually fences).
#[must_use]
pub fn extract_code_block(text: &str) -> &str {
    let Some(start) = text.find("```") else { return text };
    // Skip the fence's own line (``` or ```rust).
    let after_fence = &text[start + 3..];
    let line_end = after_fence.find('\n').unwrap_or(after_fence.len());
    let body = &after_fence[line_end..];
    match body.find("```") {
        Some(end) => body[..end].trim(),
        None => body.trim_start_matches('\n'),
    }
}

/// Normalize text for exact-match comparison: trim trailing whitespace per
/// line + drop blank lines + join with `\n` (riir-train driver semantics).
#[must_use]
pub fn normalize_text(s: &str) -> String {
    s.lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Exact match against the expected response, after symmetric fence-strip +
/// normalize. `None` when the sample carries no ground truth.
#[must_use]
pub fn exact_match(gen: &str, expected: Option<&str>) -> Option<bool> {
    expected.map(|exp| {
        normalize_text(extract_code_block(gen)) == normalize_text(extract_code_block(exp))
    })
}

/// Load v2edit eval chunks from a data dir.
///
/// # Errors
///
/// Returns a `String` (path + io/parse context) if a chunk file is missing
/// or a row fails to parse — the harness refuses to run on a silently
/// truncated eval set.
pub fn load_v2edit(data_dir: &Path, chunks: &[usize], limit: usize) -> Result<Vec<Sample>, String> {
    #[derive(serde::Deserialize)]
    struct Row {
        prompt: String,
        response: String,
        #[serde(default)]
        lint: Option<String>,
        #[serde(default)]
        lints: Option<Vec<String>>,
    }

    let mut out = Vec::new();
    for &c in chunks {
        let p = data_dir.join(format!("clippy_l4_v2edit_test_chunk_{c:02}.jsonl"));
        let raw = std::fs::read_to_string(&p)
            .map_err(|e| format!("cannot read {}: {e}", p.display()))?;
        for (i, line) in raw.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            if limit != 0 && out.len() >= limit {
                return Ok(out);
            }
            let row: Row = serde_json::from_str(line)
                .map_err(|e| format!("{}:{}: bad JSONL row: {e}", p.display(), i + 1))?;
            let mut lints = row.lints.unwrap_or_default();
            if let Some(l) = row.lint {
                lints.push(l);
            }
            if lints.is_empty() {
                return Err(format!(
                    "{}:{}: row carries neither `lint` nor `lints` — cannot target the oracle",
                    p.display(),
                    i + 1
                ));
            }
            out.push(Sample {
                id: format!("v2edit:c{c:02}:{i:03}"),
                lints,
                module: prompt_module(&row.prompt).to_string(),
                expected: Some(row.response),
                source: SampleSource::V2edit,
            });
        }
    }
    Ok(out)
}

/// The reachable-miss samples.
#[must_use]
pub fn reachable_samples() -> Vec<Sample> {
    REACHABLE
        .iter()
        .map(|(lint, code)| Sample {
            id: format!("reachable:{lint}"),
            lints: vec![(*lint).to_string()],
            module: (*code).to_string(),
            expected: None,
            source: SampleSource::Reachable,
        })
        .collect()
}

// ── Miss classification + modelless candidate ─────────────────────────────

/// How the shipped modelless pipeline treats a sample (the three L4-reachable
/// miss classes from Bench 037 + the served case).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissClass {
    /// Lint absent from the corpus — the Issue 030 relevance gate declines;
    /// this is L4's actual deployment distribution.
    OffCorpus,
    /// In corpus, `heal()` served a parsing fix (L4 never consulted).
    ServedParsing,
    /// In corpus, `heal()` served a non-parsing fix (miss #2).
    ServedNoParse,
    /// In corpus, `heal()` declined (miss #1).
    Declined,
}

impl MissClass {
    /// True when the shipped seam would consult a generative fallback.
    #[must_use]
    pub fn is_reachable_miss(self) -> bool {
        matches!(self, MissClass::OffCorpus | MissClass::ServedNoParse | MissClass::Declined)
    }
}

// ── Prompts ────────────────────────────────────────────────────────────────

/// One retrieved corpus exemplar for the pre-pass prompt.
#[derive(Debug, Clone)]
pub struct FewShot {
    pub rule_id: String,
    pub buggy: String,
    pub fixed: String,
}

/// Arm A prompt: the trained instruction + the module, verbatim — the exact
/// distribution the Bonsai adapter was tuned on, and the raw baseline for
/// the CF arms.
#[must_use]
pub fn prompt_raw(module: &str) -> String {
    format!("{TRAINED_INSTRUCTION}\n\n{module}")
}

/// Arm B/C prompt: instruction + module + the modelless candidate + corpus
/// few-shots (the Proposal 005 "modelless pre-pass → LLM" pipeline step).
///
/// Deterministic by construction: the few-shot order is the retrieval
/// ranking's order, the candidate is the seeded pipeline's serve (or an
/// explicit "declined" line), and nothing else varies.
#[must_use]
pub fn prompt_prepass(module: &str, modelless: Option<&str>, fewshots: &[FewShot]) -> String {
    let mut s = String::new();
    s.push_str(TRAINED_INSTRUCTION);
    s.push_str("\n\n");
    s.push_str(module);
    s.push_str("\n\nA static modelless analyzer proposed this candidate fix (it may be WRONG — verify it, do not trust it):\n");
    match modelless {
        Some(c) => {
            for line in c.lines() {
                s.push_str("    ");
                s.push_str(line);
                s.push('\n');
            }
        }
        None => s.push_str("    (no candidate — the modelless path declined this input)\n"),
    }
    if !fewshots.is_empty() {
        s.push_str("\nSimilar known-good fixes from the rule corpus:\n");
        for fs in fewshots {
            s.push_str("--- example (");
            s.push_str(&fs.rule_id);
            s.push_str(") before ---\n");
            s.push_str(&fs.buggy);
            s.push_str("\n--- after ---\n");
            s.push_str(&fs.fixed);
            s.push('\n');
        }
    }
    s.push_str("\nRespond with ONLY the corrected lines that differ from the input code, no explanation.");
    s
}

// ── Edit application (the v2edit "only the corrected lines" contract) ──────

/// Compute the brace depth before each line of `src`.
fn line_depths(src: &[&str]) -> Vec<i64> {
    let mut depths = Vec::with_capacity(src.len() + 1);
    let mut d = 0i64;
    depths.push(d);
    for l in src {
        for ch in l.chars() {
            match ch {
                '{' => d += 1,
                '}' => d -= 1,
                _ => {}
            }
        }
        depths.push(d);
    }
    depths
}

/// Longest-common-subsequence alignment on trimmed lines.
fn lcs_align(lhs: &[&str], rhs: &[&str]) -> Vec<(usize, usize)> {
    let (rows, cols) = (lhs.len(), rhs.len());
    let mut dp = vec![vec![0usize; cols + 1]; rows + 1];
    for row in (0..rows).rev() {
        for col in (0..cols).rev() {
            dp[row][col] = if lhs[row].trim() == rhs[col].trim() {
                dp[row + 1][col + 1] + 1
            } else {
                dp[row + 1][col].max(dp[row][col + 1])
            };
        }
    }
    let mut out = Vec::new();
    let (mut row, mut col) = (0usize, 0usize);
    while row < rows && col < cols {
        if lhs[row].trim() == rhs[col].trim() {
            out.push((row, col));
            row += 1;
            col += 1;
        } else if dp[row + 1][col] >= dp[row][col + 1] {
            row += 1;
        } else {
            col += 1;
        }
    }
    out
}

/// LCS-based merge: response lines that match original lines (trimmed) are
/// anchors; maximal non-matching response runs between anchors replace the
/// original lines in the same gap. `depth_bound` never consumes an original
/// line that would take the brace depth below the gap's starting depth (so a
/// trailing run cannot eat the enclosing item's closing braces or the rest
/// of the file); unbounded consumes the whole gap.
///
/// Returns `None` when the response cannot be located (a leading run with no
/// anchor, or no anchors at all).
fn lcs_merge(orig: &[&str], resp: &[&str], depth_bound: bool) -> Option<Vec<String>> {
    let anchors = lcs_align(orig, resp);
    if resp.is_empty() {
        return Some(orig.iter().map(|s| (*s).to_string()).collect());
    }
    if anchors.is_empty() {
        return None;
    }
    // A response run BEFORE the first anchor cannot be located.
    if anchors[0].1 != 0 {
        return None;
    }
    let depths = line_depths(orig);

    let mut out: Vec<String> = Vec::new();
    let mut oi = 0usize;
    let mut prev_rj: Option<usize> = None;

    let apply_gap = |out: &mut Vec<String>,
                         oi: &mut usize,
                         end_anchor: usize,
                         run: &[&str],
                         depths: &[i64],
                         depth_bound: bool|
     -> bool {
        if run.is_empty() {
            // No replacement content: the skipped original lines are
            // unchanged context — keep them.
            out.extend(orig[*oi..end_anchor].iter().map(|s| (*s).to_string()));
            *oi = end_anchor;
            return true;
        }
        let d0 = depths[*oi];
        let mut end = end_anchor;
        if depth_bound {
            end = *oi;
            while end < end_anchor && depths[end + 1] >= d0 {
                end += 1;
            }
        }
        out.extend(run.iter().map(|s| (*s).to_string()));
        // Lines orig[end..end_anchor] were NOT consumed (depth bound stopped
        // early) — they are unchanged context, keep them.
        out.extend(orig[end..end_anchor].iter().map(|s| (*s).to_string()));
        *oi = end_anchor;
        true
    };

    for &(ai, rj) in &anchors {
        let run: Vec<&str> = match prev_rj {
            Some(prev) => resp[prev + 1..rj].to_vec(),
            None => Vec::new(),
        };
        if !apply_gap(&mut out, &mut oi, ai, &run, &depths, depth_bound) {
            return None;
        }
        out.push(orig[ai].to_string());
        prev_rj = Some(rj);
        oi = ai + 1;
    }

    // Trailing run after the last anchor.
    let tail: Vec<&str> = resp[prev_rj? + 1..].to_vec();
    if tail.is_empty() {
        out.extend(orig[oi..].iter().map(|s| (*s).to_string()));
    } else {
        let d0 = depths[oi];
        let mut end = orig.len();
        if depth_bound {
            end = oi;
            while end < orig.len() && depths[end + 1] >= d0 {
                end += 1;
            }
        }
        out.extend(tail.iter().map(|s| (*s).to_string()));
        out.extend(orig[end..].iter().map(|s| (*s).to_string()));
    }
    Some(out)
}

/// Find the span of the top-level `fn name(...) {...}` in `src` via brace
/// matching (riir-train `rlvr_reward::find_fn_span` semantics, copied per
/// the t51 precedent — riir-train is not an allowed dep).
fn find_fn_span(src: &str, name: &str) -> Option<(usize, usize)> {
    let pat = format!("fn {name}");
    let bytes = src.as_bytes();
    let mut search = 0usize;
    while let Some(rel) = src[search..].find(&pat) {
        let at = search + rel;
        search = at + 1;
        let left_ok = at == 0
            || !(bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_');
        let after = at + pat.len();
        let right_ok =
            after >= bytes.len() || !(bytes[after].is_ascii_alphanumeric() || bytes[after] == b'_');
        if !left_ok || !right_ok {
            continue;
        }
        let Some(brace) = src[at..].find('{') else { continue };
        let brace = at + brace;
        let mut depth = 0usize;
        for (i, ch) in src[brace..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((at, brace + i + 1));
                    }
                }
                _ => {}
            }
        }
        return None;
    }
    None
}

fn response_fn_name(block: &str) -> Option<&str> {
    let b = block.trim_start();
    let b = b.strip_prefix("pub ").unwrap_or(b);
    let rest = b.strip_prefix("fn ")?.trim_start();
    let n = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .count();
    if n == 0 { None } else { Some(&rest[..n]) }
}

/// Whole-fn substitution (riir-train `rlvr_reward::substitute` semantics):
/// every `\n\n`-separated response block must be a `fn` present in the
/// module; each replaces its whole span.
fn substitute_fn_spans(module: &str, response: &str) -> Option<String> {
    let mut out = module.to_string();
    for block in response.split("\n\n") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }
        let name = response_fn_name(block)?;
        let span = find_fn_span(&out, name)?;
        out.replace_range(span.0..span.1, block);
    }
    Some(out)
}

/// Which merge strategy produced a result (diagnostic column — a strategy
/// distribution shift between arms is itself a finding).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// The generation is a verbatim full replacement (reachable fixtures
    /// only — never tried for v2edit, where it would drop the other fns).
    Verbatim,
    /// Whole-fn span substitution.
    FnSpans,
    /// LCS merge, depth-bounded.
    LcsBounded,
    /// LCS merge, unbounded.
    LcsUnbounded,
}

impl MergeStrategy {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            MergeStrategy::Verbatim => "verbatim",
            MergeStrategy::FnSpans => "fn-spans",
            MergeStrategy::LcsBounded => "lcs-bounded",
            MergeStrategy::LcsUnbounded => "lcs-unbounded",
        }
    }
}

/// Apply an edit-style response to a module. Strategies are tried in order
/// and the FIRST whose result parses as a Rust file wins; the strategy is
/// reported so a distribution shift between arms is visible.
///
/// `verbatim_ok` should be false whenever the module contains items the
/// response does not repeat (v2edit multi-fn modules) — a verbatim replace
/// there loses content and fakes a findings reduction.
#[must_use]
pub fn apply_edit(module: &str, response: &str, verbatim_ok: bool) -> Option<(String, MergeStrategy)> {
    let gen = extract_code_block(response);
    let parses = |s: &str| syn::parse_file(s).is_ok();

    if verbatim_ok {
        let t = gen.trim();
        if !t.is_empty() && parses(t) {
            return Some((t.to_string(), MergeStrategy::Verbatim));
        }
    }
    if let Some(m) = substitute_fn_spans(module, gen) {
        if parses(&m) {
            return Some((m, MergeStrategy::FnSpans));
        }
    }
    let orig: Vec<&str> = module.lines().collect();
    let resp: Vec<&str> = gen.lines().collect();
    for (bounded, strat) in [(true, MergeStrategy::LcsBounded), (false, MergeStrategy::LcsUnbounded)] {
        if let Some(lines) = lcs_merge(&orig, &resp, bounded) {
            let m = lines.join("\n");
            if parses(&m) {
                return Some((m, strat));
            }
        }
    }
    None
}

// ── Contract preservation (destructive-edit veto, Bench 039) ───────────────

/// Identifiers that form an item's public contract (fn/param/type/field/
/// variant names). Deliberately EXCLUDES local `let` bindings — removing a
/// local is the legitimate fix for several lints (`redundant_clone`).
///
/// Copied from `tests/t51_l4_vs_decline.rs` (syn visitor) per the
/// copy-don't-share precedent; see that file for the design rationale.
#[must_use]
pub fn contract_idents(code: &str) -> BTreeSet<String> {
    use syn::visit::Visit;

    #[derive(Default)]
    struct V(BTreeSet<String>);

    impl<'ast> Visit<'ast> for V {
        fn visit_signature(&mut self, sig: &'ast syn::Signature) {
            self.0.insert(format!("fn:{}", sig.ident));
            for arg in &sig.inputs {
                if let syn::FnArg::Typed(pt) = arg {
                    if let syn::Pat::Ident(pi) = &*pt.pat {
                        let name = pi.ident.to_string();
                        if !name.starts_with('_') {
                            self.0.insert(format!("arg:{}:{}", sig.ident, name));
                        }
                    }
                }
            }
            syn::visit::visit_signature(self, sig);
        }
        fn visit_field(&mut self, f: &'ast syn::Field) {
            if let Some(i) = &f.ident {
                self.0.insert(format!("field:{i}"));
            }
            syn::visit::visit_field(self, f);
        }
        fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
            self.0.insert(format!("type:{}", i.ident));
            syn::visit::visit_item_struct(self, i);
        }
        fn visit_item_enum(&mut self, i: &'ast syn::ItemEnum) {
            self.0.insert(format!("type:{}", i.ident));
            syn::visit::visit_item_enum(self, i);
        }
        fn visit_variant(&mut self, v: &'ast syn::Variant) {
            self.0.insert(format!("variant:{}", v.ident));
            syn::visit::visit_variant(self, v);
        }
    }

    let mut v = V::default();
    // Same two-strategy parse the pruner uses, so this sees exactly what the
    // pruner accepted.
    if let Ok(f) = syn::parse_file(code) { v.visit_file(&f) } else if let Ok(f) = syn::parse_file(&format!("fn _wrap() {{\n{code}\n}}")) {
                v.visit_file(&f);
            }
    v.0
}

/// True when the fix drops no contract identifier the input had (additions
/// are fine — only losses are destructive). Bench 039 route-2 veto.
#[must_use]
pub fn contract_preserved(buggy: &str, fixed: &str) -> bool {
    contract_idents(buggy).is_subset(&contract_idents(fixed))
}

/// First `fn NAME` in the source — the behavioural-equivalence name hint.
#[must_use]
pub fn first_fn_name(code: &str) -> Option<String> {
    let mut rest = code;
    while let Some(at) = rest.find("fn ") {
        let after = &rest[at + 3..];
        let name: String = after
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            return Some(name);
        }
        rest = after;
    }
    None
}

// ── CF Workers AI client ───────────────────────────────────────────────────

/// A generation result with the accounting the metrics need.
#[derive(Debug, Clone)]
pub struct CfGen {
    pub text: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub latency_ms: u128,
}

/// Minimal blocking CF Workers AI REST client.
///
/// The token is held privately and NEVER appears in errors or Debug output.
pub struct CfClient {
    agent: ureq::Agent,
    url: String,
    token: String,
}

impl std::fmt::Debug for CfClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately no token, no url (url embeds the account id).
        f.debug_struct("CfClient").finish_non_exhaustive()
    }
}

impl CfClient {
    /// Default model (the issue's arm A/B choice).
    pub const DEFAULT_MODEL: &'static str = "@cf/meta/llama-3.3-70b-instruct-fp8-fast";

    /// Build from `CF_API_TOKEN` + `CF_ACCOUNT_ID` (+ optional model override).
    ///
    /// # Errors
    ///
    /// A human-readable message naming exactly which env var is missing —
    /// never the token value.
    pub fn from_env(model: Option<&str>) -> Result<Self, String> {
        let token = std::env::var("CF_API_TOKEN")
            .map_err(|_| "CF_API_TOKEN is not set — arms A/B need it (export CF_API_TOKEN=...)".to_string())?;
        let account = std::env::var("CF_ACCOUNT_ID").map_err(|_| {
            "CF_ACCOUNT_ID is not set — arms A/B need it (export CF_ACCOUNT_ID=...)".to_string()
        })?;
        let model = model.unwrap_or(Self::DEFAULT_MODEL);
        Ok(Self {
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_mins(3))
                .build(),
            url: format!(
                "https://api.cloudflare.com/client/v4/accounts/{account}/ai/run/{model}"
            ),
            token,
        })
    }

    /// One generation. Temperature 0 for reproducibility.
    ///
    /// # Errors
    ///
    /// Transport/HTTP/shape errors — body text on HTTP failure (CF error
    /// JSON), never the token.
    pub fn generate(&self, prompt: &str, max_tokens: usize) -> Result<CfGen, String> {
        let t0 = std::time::Instant::now();
        let body = serde_json::json!({
            "messages": [{ "role": "user", "content": prompt }],
            "max_tokens": max_tokens,
            "temperature": 0.0,
        });
        let resp = self
            .agent
            .post(&self.url)
            .set("Authorization", &format!("Bearer {}", self.token))
            .send_json(body)
            .map_err(|e| format!("CF request failed: {e}"))?;
        let v: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("CF response was not JSON: {e}"))?;
        let result = v
            .get("result")
            .ok_or_else(|| format!("CF response had no result: {v}"))?;
        let text = match result.get("response") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Array(items)) => items
                .iter()
                .filter_map(|i| i.as_str())
                .collect::<Vec<_>>()
                .join(""),
            _ => return Err(format!("CF result had no response field: {result}")),
        };
        let usage = result.get("usage");
        let get = |k: &str| {
            usage
                .and_then(|u| u.get(k))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
        };
        Ok(CfGen {
            text,
            prompt_tokens: get("prompt_tokens"),
            completion_tokens: get("completion_tokens"),
            latency_ms: t0.elapsed().as_millis(),
        })
    }
}

// ── Scoring ────────────────────────────────────────────────────────────────

/// Behavioural-equivalence summary for a kept fix (the Bench 039 "route 3"
/// veto — silent logic changes that pass every name-based check). Serialize:
/// the JSONL per-case record carries it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum BehavSummary {
    Equivalent { cases: usize },
    Divergent { input: String },
    Unsupported { reason: String },
}

impl BehavSummary {
    fn from_verdict(v: &Verdict) -> Self {
        match v {
            Verdict::Equivalent { cases } => BehavSummary::Equivalent { cases: *cases },
            Verdict::Divergent { input, .. } => BehavSummary::Divergent {
                input: input.clone(),
            },
            Verdict::Unsupported { reason } => BehavSummary::Unsupported {
                reason: reason.clone(),
            },
        }
    }
}

/// Per-case partial-credit score. The bools are independent verdict axes,
/// not a config — grouping them into bitflags would obscure the report.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct CaseScore {
    pub id: String,
    /// EM vs the expected response (v2edit only).
    pub em: Option<bool>,
    /// The L2 seam gate on the RAW generation (fragment parse — the known
    /// edit-only contract property).
    pub fragment_parses: bool,
    /// Which merge strategy applied (None = merge failed).
    pub merge_strategy: Option<MergeStrategy>,
    /// The buggy module trips a target lint in the verifier scaffold.
    pub control_fires: bool,
    /// `cargo clippy` ran on both before + merged.
    pub oracle_ran: bool,
    pub findings_before: usize,
    pub findings_after: usize,
    /// Strictly fewer clippy findings after the fix (the never-wrong keep
    /// rule's core test).
    pub strictly_fewer: bool,
    /// The sample's FIRST target lint is gone.
    pub target_resolved: bool,
    /// Generation is a no-op (echoes the input).
    pub noop: bool,
    /// Contract preservation (destructive-edit veto).
    pub contract_ok: bool,
    /// Behavioural equivalence (logic-change veto), reachable cases only.
    pub behavioural: Option<BehavSummary>,
    /// The proposal's verify filter: control + ran + strictly fewer + not a
    /// no-op (`!noop`).
    pub keep_oracle: bool,
    /// Hardened filter: `keep_oracle` AND both vetoes hold (not divergent,
    /// not destructive).
    pub keep_strict: bool,
}

/// Score one generation against one sample.
///
/// `behav_workdir` enables the behavioural-equivalence check (reachable
/// cases only — it compiles a differential driver with rustc per case).
#[must_use]
pub fn score_case(
    verifier: &ClippyVerifier,
    sample: &Sample,
    generation: &str,
    behav_workdir: Option<&Path>,
) -> CaseScore {
    let pruner = RustAnalyzerPruner::new();
    let gen = extract_code_block(generation);
    let noop = gen.trim() == sample.module.trim();
    let fragment_parses = pruner.validate(gen, &sample.lints[0]).parses;

    // Control: the BUGGY input must trip a target lint in the same scaffold
    // (t51 `control_fires` — without it a non-compiling crate reads as
    // "resolved" because clippy emits no lints at all).
    let before = verifier.verify(&sample.module, &sample.lints[0]);
    let control_fires =
        before.ran && before.present_lints.iter().any(|l| l == &format!("clippy::{}", sample.lints[0]));

    let (merged, strategy) =
        match apply_edit(&sample.module, generation, sample.source == SampleSource::Reachable) {
            Some((m, s)) => (Some(m), Some(s)),
            None => (None, None),
        };

    let mut s = CaseScore {
        id: sample.id.clone(),
        em: exact_match(generation, sample.expected.as_deref()),
        fragment_parses,
        merge_strategy: strategy,
        control_fires,
        oracle_ran: false,
        findings_before: before.present_lints.len(),
        findings_after: usize::MAX,
        strictly_fewer: false,
        target_resolved: false,
        noop,
        contract_ok: false,
        behavioural: None,
        keep_oracle: false,
        keep_strict: false,
    };

    let Some(merged) = merged else { return s };
    s.contract_ok = contract_preserved(&sample.module, &merged);
    let after = verifier.verify(&merged, &sample.lints[0]);
    s.oracle_ran = before.ran && after.ran;
    s.findings_after = after.present_lints.len();
    s.strictly_fewer = s.oracle_ran && s.findings_after < s.findings_before;
    s.target_resolved = s.oracle_ran
        && !after
            .present_lints
            .iter()
            .any(|l| l == &format!("clippy::{}", sample.lints[0]));

    s.keep_oracle = s.control_fires && s.oracle_ran && s.strictly_fewer && !s.noop;

    // Behavioural veto: only worth the rustc cost when the oracle filter
    // already passed and we have a workdir. Single-item fixtures only
    // (reachable); v2edit multi-fn modules report None.
    if s.keep_oracle && sample.source == SampleSource::Reachable {
        if let (Some(dir), Some(name)) = (behav_workdir, first_fn_name(&sample.module)) {
            let case_dir = dir.join(sample.id.replace(':', "_"));
            let v = behavioural_equivalence::check(&sample.module, &merged, &name, 64, &case_dir);
            s.behavioural = Some(BehavSummary::from_verdict(&v));
        }
    }

    let not_divergent = !matches!(s.behavioural, Some(BehavSummary::Divergent { .. }));
    s.keep_strict = s.keep_oracle && s.contract_ok && not_divergent;
    s
}

// ── The harness ────────────────────────────────────────────────────────────

/// Shared state for one POC run: the seeded pipeline (miss classification +
/// modelless candidate), the retrieval few-shot source, the oracle.
pub struct RegateHarness {
    pub pipeline: HealPipeline,
    pub corpus: ClippyCorpus,
    pub matcher: LatentMatcher,
    pub verifier: ClippyVerifier,
}

impl RegateHarness {
    /// Build from the real seed corpus (asserting the reachable premise).
    ///
    /// # Errors
    ///
    /// `LatentMatcher::from_enabled_domains` failure (index build).
    ///
    /// # Panics
    ///
    /// Panics when the corpus BLAKE3 commitment fails to verify or a
    /// reachable lint has grown a corpus entry (the premise assert — see
    /// [`assert_reachable_premise`]).
    pub fn new() -> Result<Self, String> {
        let corpus = crate::build_seed_corpus();
        assert!(corpus.verify(), "corpus BLAKE3 commitment must verify");
        assert_reachable_premise(&corpus);
        let mut pipeline =
            HealPipeline::new(crate::TernaryDrafter::fallback(), RustAnalyzerPruner::new());
        pipeline.seed_from_corpus(&corpus.entries);
        let matcher = LatentMatcher::from_enabled_domains()?;
        Ok(Self {
            pipeline,
            corpus,
            matcher,
            verifier: ClippyVerifier::new(),
        })
    }

    /// Classify a sample's miss class AND capture the modelless candidate
    /// (one `heal()` call serves both).
    ///
    /// The candidate list is `corpus.lookup(lint)` — the `bench_003`/`t51`
    /// construction (the shipped configuration): passing the FULL entry list
    /// instead lets the drafter fabricate serves for off-corpus rules off
    /// unrelated entries, which measures a pipeline nobody ships.
    pub fn classify(&mut self, lint: &str, module: &str) -> (MissClass, Option<String>) {
        let candidates: Vec<crate::ClippyFixEntry> =
            self.corpus.lookup(lint).cloned().collect();
        let in_corpus = !candidates.is_empty();
        if let Some(r) = self.pipeline.heal(lint, module, &candidates) {
                let class = if in_corpus {
                    if r.parses {
                        MissClass::ServedParsing
                    } else {
                        MissClass::ServedNoParse
                    }
                } else {
                    // Post-Issue-030 the relevance gate declines off-corpus
                    // input; a serve here means the gate regressed.
                    MissClass::OffCorpus
                };
                (class, Some(r.fixed_code))
            } else {
                let class = if in_corpus {
                    MissClass::Declined
                } else {
                    MissClass::OffCorpus
                };
                (class, None)
            }
    }

    /// Top-k corpus few-shots for a module (the `--suggest` ranking, rule
    /// dedup, fixed order). The CLIPPY domain's fan-out arm only — this is a
    /// clippy fix task and perf/kernel exemplars are shape-similar noise.
    ///
    /// The target lint's own corpus entry is deliberately INCLUDED when it
    /// exists: arm B is the retrieval-augmented product pipeline, and on the
    /// reachable set no self-entry exists by construction — the premise is
    /// asserted at harness build.
    pub fn few_shots(&mut self, module: &str, k: usize) -> Vec<FewShot> {
        let (_routed, by_domain) = self.matcher.suggest_span(module, k * 2);
        let ranked = by_domain
            .into_iter()
            .find(|(d, _)| matches!(d, DomainId::Clippy))
            .map(|(_, rules)| rules)
            .unwrap_or_default();
        let mut out: Vec<FewShot> = Vec::new();
        for r in ranked {
            if out.len() >= k {
                break;
            }
            if out.iter().any(|f| f.rule_id == r.rule_id) {
                continue;
            }
            if let Some(info) = self.matcher.rule_book(&r.rule_id) {
                out.push(FewShot {
                    rule_id: r.rule_id.clone(),
                    buggy: info.buggy_code.clone(),
                    fixed: info.fixed_code.clone(),
                });
            }
        }
        out
    }
}

// ── Aggregation ────────────────────────────────────────────────────────────

/// Per-arm aggregate.
#[derive(Debug, Default)]
pub struct ArmSummary {
    pub n: usize,
    pub em_hits: usize,
    pub em_total: usize,
    pub fragment_parses: usize,
    pub merge_ok: usize,
    pub control_fires: usize,
    pub oracle_scoreable: usize,
    pub strictly_fewer: usize,
    pub keep_oracle: usize,
    pub keep_strict: usize,
    /// `keep_oracle` cases vetoed by contract or behavioural checks.
    pub vetoed: usize,
    pub divergent: usize,
    /// Oracle-scoreable cases that served a fix but did NOT reduce findings
    /// (what decline would have avoided).
    pub regressions: usize,
    pub reachable_n: usize,
    pub reachable_keep_strict: usize,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost_usd: f64,
    pub latency_ms_total: u128,
    pub latency_ms_max: u128,
}

impl ArmSummary {
    /// Fold one case in. `reachable` flags reachable-miss-set membership.
    pub fn add(&mut self, s: &CaseScore, reachable: bool, prompt_tokens: u64, completion_tokens: u64, price_in_per_m: f64, price_out_per_m: f64) {
        self.n += 1;
        if reachable {
            self.reachable_n += 1;
        }
        if let Some(em) = s.em {
            self.em_total += 1;
            if em {
                self.em_hits += 1;
            }
        }
        if s.fragment_parses {
            self.fragment_parses += 1;
        }
        if s.merge_strategy.is_some() {
            self.merge_ok += 1;
        }
        if s.control_fires {
            self.control_fires += 1;
        }
        let scoreable = s.control_fires && s.oracle_ran && s.merge_strategy.is_some();
        if scoreable {
            self.oracle_scoreable += 1;
            if s.strictly_fewer {
                self.strictly_fewer += 1;
            } else if !s.noop {
                self.regressions += 1;
            }
        }
        if s.keep_oracle {
            self.keep_oracle += 1;
            let holds = s.contract_ok
                && !matches!(s.behavioural, Some(BehavSummary::Divergent { .. }));
            if !holds {
                self.vetoed += 1;
            }
        }
        if matches!(s.behavioural, Some(BehavSummary::Divergent { .. })) {
            self.divergent += 1;
        }
        if s.keep_strict {
            self.keep_strict += 1;
            if reachable {
                self.reachable_keep_strict += 1;
            }
        }
        self.prompt_tokens += prompt_tokens;
        self.completion_tokens += completion_tokens;
        // Cost accounting is a report figure, not a sync value — the u64 →
        // f64 precision loss is irrelevant at token counts (< 2^32).
        #[allow(clippy::cast_precision_loss)]
        let cost = prompt_tokens as f64 / 1e6 * price_in_per_m
            + completion_tokens as f64 / 1e6 * price_out_per_m;
        self.cost_usd += cost;
    }

    /// The pre-registered verdict line: acceptance vs decline on the
    /// reachable-miss set (decline accepts 0 by definition; the harm side —
    /// kept-but-wrong — is what the vetoes + strict-keep report).
    #[must_use]
    pub fn verdict_vs_decline(&self) -> String {
        if self.reachable_n == 0 {
            return "no reachable-miss cases in this arm".to_string();
        }
        let beats = self.reachable_keep_strict >= 1;
        format!(
            "reachable-miss strict-keep: {}/{} vs decline 0/{} → {}",
            self.reachable_keep_strict,
            self.reachable_n,
            self.reachable_n,
            if beats { "BEATS decline" } else { "DOES NOT beat decline" }
        )
    }

    /// Markdown table row.
    #[must_use]
    pub fn markdown_row(&self, arm: &str) -> String {
        format!(
            "| {arm} | {} | {}/{} | {} | {} | {}/{} | {} | {} | {} | {} | {:.4} |",
            self.n,
            self.em_hits,
            self.em_total,
            self.fragment_parses,
            self.merge_ok,
            self.keep_oracle,
            self.oracle_scoreable,
            self.keep_strict,
            self.vetoed,
            self.divergent,
            self.regressions,
            self.cost_usd,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── prompt_module / extract / normalize ──────────────────────────────

    #[test]
    fn prompt_module_strips_the_instruction() {
        let p = format!("{TRAINED_INSTRUCTION}\n\nfn a() {{}}");
        assert_eq!(prompt_module(&p), "fn a() {}");
        assert_eq!(prompt_module("fn a() {}"), "fn a() {}");
    }

    #[test]
    fn extract_code_block_handles_fences_and_bare() {
        assert_eq!(extract_code_block("```rust\nfn a() {}\n```"), "fn a() {}");
        assert_eq!(extract_code_block("plain"), "plain");
        assert_eq!(extract_code_block("```\nunclosed"), "unclosed");
    }

    #[test]
    fn normalize_drops_blank_lines_and_trailing_ws() {
        assert_eq!(normalize_text("a  \n\nb\n"), "a\nb");
    }

    #[test]
    fn exact_match_is_fence_symmetric() {
        assert_eq!(exact_match("```rust\nfn a() {}\n```", Some("fn a() {}")), Some(true));
        assert_eq!(exact_match("fn b() {}", Some("fn a() {}")), Some(false));
        assert_eq!(exact_match("fn a() {}", None), None);
    }

    // ── merge ─────────────────────────────────────────────────────────────

    /// The REAL first row of v2edit chunk 00 (read from the eval corpus on
    /// 2026-08-25): a two-site edit with omitted unchanged closing braces —
    /// the exact shape whole-fn substitution cannot apply but the LCS merge
    /// must.
    const V2EDIT_ROW: &str = "fn fetch(s: &str) -> usize {\n    s.split_whitespace().count()\n}\n\nfn hash_label(v: i32) -> i64 { v as i64 }\nfn baz(x: Option<i32>) -> Option<i64> {\n    match x {\n        Some(v) => Some(hash_label(v)),\n        None => None,\n    }\n}\n\nfn review(v: &[u32]) -> Option<u32> {\n    v.iter().copied().min()\n}\n\nfn show(x: &i32) -> i32 { *x }\nfn call(x: &i32) -> i32 {\n    show(&x)\n}";
    const V2EDIT_RESP: &str = "fn baz(x: Option<i32>) -> Option<i64> {\n    x.map(hash_label)\nfn call(x: &i32) -> i32 {\n    show(x)";

    #[test]
    fn lcs_merge_applies_the_real_v2edit_reference_row() {
        let orig: Vec<&str> = V2EDIT_ROW.lines().collect();
        let resp: Vec<&str> = V2EDIT_RESP.lines().collect();
        let merged = lcs_merge(&orig, &resp, true).expect("depth-bounded merge must apply");
        let m = merged.join("\n");
        assert!(m.contains("x.map(hash_label)"), "baz body replaced: {m}");
        assert!(m.contains("show(x)"), "call body replaced: {m}");
        assert!(!m.contains("show(&x)"), "old call body gone: {m}");
        // The untouched items survive (verbatim replacement would drop them).
        assert!(m.contains("fn fetch"));
        assert!(m.contains("fn review"));
        assert!(m.contains("fn hash_label"));
        // Replacing a `match {}` block with a single expression legitimately
        // removes a brace PAIR — the invariant is net balance, not count.
        assert_eq!(m.matches('{').count(), m.matches('}').count(), "net brace balance");
        assert!(syn::parse_file(&m).is_ok(), "merged result must parse");
    }

    #[test]
    fn apply_edit_picks_fn_spans_for_a_full_fn_response() {
        let module = "pub fn f(v: &Vec<u8>) -> usize { v.len() }";
        let resp = "pub fn f(v: &[u8]) -> usize { v.len() }";
        let (m, s) = apply_edit(module, resp, true).expect("must merge");
        assert_eq!(s, MergeStrategy::Verbatim);
        assert!(m.contains("&[u8]"));
    }

    #[test]
    fn apply_edit_refuses_verbatim_for_v2edit_sources() {
        // A complete-fn generation against a multi-fn module must NOT take
        // the verbatim path (it would drop the other fns).
        let (m, s) = apply_edit(V2EDIT_ROW, "fn fetch(s: &str) -> usize {\n    s.split_whitespace().count()\n}", false)
            .expect("must merge via a non-verbatim strategy");
        assert_ne!(s, MergeStrategy::Verbatim);
        assert!(m.contains("fn baz"), "other fns survive: {m}");
    }

    #[test]
    fn apply_edit_fails_loudly_on_garbage() {
        assert!(apply_edit("fn a() {}", "}}}garbage{{{", true).is_none());
    }

    #[test]
    fn lcs_merge_leading_run_is_unlocatable() {
        let orig: Vec<&str> = "fn a() {\n}".lines().collect();
        let resp: Vec<&str> = "fn b() {\n}".lines().collect();
        assert!(lcs_merge(&orig, &resp, true).is_none());
    }

    #[test]
    fn depth_bound_stops_at_the_enclosing_brace() {
        // A trailing run must not eat the rest of the file.
        let orig = "fn a() {\n    old();\n}\nfn b() {\n    keep();\n}".to_string();
        let orig_lines: Vec<&str> = orig.lines().collect();
        let resp: Vec<&str> = "fn a() {\n    new();".lines().collect();
        let merged = lcs_merge(&orig_lines, &resp, true).expect("must apply");
        let m = merged.join("\n");
        assert!(m.contains("new();"));
        assert!(m.contains("keep();"), "trailing items survive: {m}");
    }

    // ── prompts ────────────────────────────────────────────────────────────

    #[test]
    fn prompt_raw_is_the_trained_distribution() {
        let p = prompt_raw("fn a() {}");
        assert!(p.starts_with("Fix all clippy lints in this Rust code."));
        assert!(p.ends_with("fn a() {}"));
    }

    #[test]
    fn prompt_prepass_carries_candidate_and_fewshots_deterministically() {
        let fs = vec![FewShot {
            rule_id: "manual_map".into(),
            buggy: "b".into(),
            fixed: "f".into(),
        }];
        let with = prompt_prepass("MOD", Some("CAND"), &fs);
        assert!(with.starts_with(TRAINED_INSTRUCTION));
        assert!(with.contains("MOD"));
        assert!(with.contains("CAND"));
        assert!(with.contains("example (manual_map) before"));
        assert!(with.contains("--- after ---"));
        let again = prompt_prepass("MOD", Some("CAND"), &fs);
        assert_eq!(with, again, "prompt construction must be deterministic");

        let declined = prompt_prepass("MOD", None, &[]);
        assert!(declined.contains("(no candidate"));
        assert!(!declined.contains("example ("));
    }

    // ── contract + first_fn ────────────────────────────────────────────────

    #[test]
    fn contract_preserved_flags_a_dropped_fn_but_not_a_dropped_local() {
        let destructive = contract_preserved(
            "pub fn f(a: u8) {}\npub fn g() {}",
            "pub fn f(a: u8) {}",
        );
        assert!(!destructive);
        let local_fix = contract_preserved(
            "pub fn f() { let s2 = 1; let _ = s2; }",
            "pub fn f() { }",
        );
        assert!(local_fix, "dropping a local let is a legitimate fix");
    }

    #[test]
    fn first_fn_name_finds_the_hint() {
        assert_eq!(first_fn_name("pub struct S;\npub fn mk() -> S { S }").as_deref(), Some("mk"));
        assert!(first_fn_name("pub enum E { A }").is_none());
    }

    // ── reachable premise (fast — no cargo, no oracle) ────────────────────

    #[test]
    fn reachable_premise_holds_against_the_seed_corpus() {
        let corpus = crate::build_seed_corpus();
        assert_reachable_premise(&corpus);
    }
}

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
//! | A | CF `@cf/qwen/qwen3.8-27b` (REST) | none ([`prompt_raw`]) |
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
use std::path::{Path, PathBuf};
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

// ── Issue 049: the growing-input channels (Meta^n conditioning POC) ──────

/// Channel 1 — one prior trajectory outcome for the queried shape
/// (retrieved from the durable store; Certified first, Withdrawn excluded
/// by `TrajStore::retrieve` itself).
#[derive(Debug, Clone)]
pub struct TrajectoryExemplar {
    pub lint_key: String,
    pub shape: String,
    pub fix: String,
    pub success: u32,
    pub fail: u32,
    /// `"Certified"` / `"Heuristic"` (Withdrawn never reaches the prompt).
    pub tier: &'static str,
}

/// Channel 3 — one per-rule failure-pattern row (the depth≥3
/// structured-summary analog: the store's aggregate verdict distribution,
/// not one span's history).
#[derive(Debug, Clone)]
pub struct FailurePattern {
    pub rule: String,
    pub attempts: u32,
    /// Kept attempts (`ok`) — carried as the integer so the prompt renders
    /// `kept/attempts` without a float round-trip (the rate stays display).
    pub kept: u32,
    pub success_rate: f32,
    /// Up to 2 representative (`shape`, attempted-fix) pairs with failures.
    pub representative_failures: Vec<(String, String)>,
}

/// Arm G prompt (Issue 049 T1): the arm-B pre-pass prompt + the two
/// growing-input channels — (1) prior trajectory outcomes for shape-similar
/// inputs, (3) the cross-rule failure-pattern summary. Channel 2 (the
/// modelless candidate) is arm B's own addition and carries through.
///
/// The `Meta`^n mechanism (R512): "the gain comes from giving the improver
/// more to read" — inter-layer conditioning carried ~72% of recursion gain
/// in the paper's ablation. This is the single-call instantiation: the
/// improver's input grows with the store; arm A/B remain byte-identical
/// baselines (their builders are untouched).
#[must_use]
pub fn prompt_growing(
    module: &str,
    modelless: Option<&str>,
    fewshots: &[FewShot],
    trajectories: &[TrajectoryExemplar],
    failure_summary: &[FailurePattern],
) -> String {
    use std::fmt::Write as _;
    let s = prompt_prepass(module, modelless, fewshots);
    // Splice the channels BEFORE the closing respond-line.
    let closing = "\nRespond with ONLY the corrected lines that differ from the input code, no explanation.";
    let base = s.strip_suffix(closing).unwrap_or(&s).to_string();
    let mut s = base;
    if !trajectories.is_empty() {
        s.push_str("\nPrior fix attempts on shape-similar code (from the trajectory store — outcomes are ground truth, not suggestions):\n");
        for t in trajectories {
            let _ = write!(
                s,
                "--- {} [{}: {} ok / {} failed] ---\n before:\n{}\n after:\n{}",
                t.lint_key, t.tier, t.success, t.fail, indent4(&t.shape), indent4(&t.fix)
            );
            s.push('\n');
        }
    }
    if !failure_summary.is_empty() {
        s.push_str("\nHistorical fix-reliability by rule (what usually works vs usually fails — calibrate against this):\n");
        for f in failure_summary {
            let _ = write!(
                s,
                "  {}: {}/{} attempts kept ({:.0}%)",
                f.rule,
                f.kept,
                f.attempts,
                f.success_rate * 100.0
            );
            s.push('\n');
            for (shape, fix) in &f.representative_failures {
                let _ = write!(
                    s,
                    "    failed attempt — before:\n{}\n    after (REJECTED):\n{}",
                    indent4(shape),
                    indent4(fix)
                );
                s.push('\n');
            }
        }
    }
    s.push_str(closing);
    s
}

fn indent4(text: &str) -> String {
    text.lines().map(|l| format!("    {l}")).collect::<Vec<_>>().join("\n")
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
    /// Per-response neuron cost when the API reports it (the qwen3.8
    /// chat.completion schema includes `usage.neurons`; legacy-schema models
    /// report 0 — use the GraphQL daily aggregate for those).
    pub neurons: f64,
    /// Thinking-token size diagnostic (reasoning-model field length —
    /// `message.reasoning` for qwen3.x, `message.reasoning_content`
    /// (DeepSeek-style) for GLM; billed as completion tokens; 0 when the
    /// model doesn't reason or thinking is disabled).
    pub reasoning_chars: usize,
    /// Whether the response carried a `usage` object (the reconcile half of
    /// the USD cap gate — Issue 046: a billed response with no usage spends
    /// the FULL reserve, never releases it).
    pub usage_reported: bool,
    pub latency_ms: u128,
}

/// Overall per-call ceiling — a reasoning model at a 2k budget can legitimately
/// take minutes.
const CALL_TIMEOUT: Duration = Duration::from_mins(3);
/// Connect-phase ceiling, load-bearing SEPARATELY from [`CALL_TIMEOUT`]: under a
/// single overall timeout a blackholed TCP connect is indistinguishable from a
/// slow model, so it stalls for the full 3 min — × every span in the window.
/// Connecting is fast or broken; nothing in between. (Issue 051 T10 fixed the
/// same defect in the free leg; the fallback leg had it too.)
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Redaction placeholder, should the token ever surface in a payload.
const TOKEN_REDACTED: &str = "<CF_API_TOKEN redacted>";
/// Byte ceiling on a server-controlled payload entering an error string.
const MAX_PAYLOAD_IN_ERROR: usize = 512;

/// Clip a server-controlled payload before it enters an error string or log.
///
/// An unbounded `{response_json}` dump floods cron logs (a large failed
/// response is unbounded in principle) and echoes content back out of the
/// process. Keeping a bounded prefix preserves every diagnostic these
/// messages are actually read for.
#[must_use]
pub fn clip_payload(s: &str) -> String {
    if s.len() <= MAX_PAYLOAD_IN_ERROR {
        return s.to_string();
    }
    // Respect char boundaries — a byte-slice of UTF-8 would panic.
    let cut = (0..=MAX_PAYLOAD_IN_ERROR)
        .rev()
        .find(|&i| s.is_char_boundary(i))
        .unwrap_or(0);
    format!("{}… (clipped, {} bytes total)", &s[..cut], s.len())
}

/// Minimal blocking CF Workers AI REST client.
///
/// The token is held privately and NEVER appears in errors or Debug output.
pub struct CfClient {
    agent: ureq::Agent,
    url: String,
    /// The pre-formatted `Bearer <token>` header value, built ONCE.
    ///
    /// Perf: the two request paths no longer allocate a header String per
    /// call. Security: the secret occupies ONE heap location for the
    /// client's lifetime instead of being copied into a fresh allocation per
    /// request and dropped un-zeroed. Mirrors `groq::GroqClient` (Issue 051
    /// T10) — same defect class, same fix, both legs of the ladder.
    auth: String,
    /// The bare token, kept ONLY to scrub it out of server-controlled
    /// strings (see [`CfClient::scrub`]).
    token: String,
    account: String,
    model: String,
    /// The USD daily-cap ledger (Issue 046) — reserve-then-reconcile on
    /// every `generate`. `&mut self` on generate is load-bearing: the gate
    /// mutates persisted state.
    spend: crate::llm_spend::SpendLedger,
}

impl std::fmt::Debug for CfClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately no token, no url (url embeds the account id).
        f.debug_struct("CfClient").finish_non_exhaustive()
    }
}

impl CfClient {
    /// Remove the token from a string bound for a log or error. Defense in
    /// depth for the paths that don't exist yet (a proxy inlining the header
    /// into a 502 page, a future query-param-auth endpoint). A leaked key in
    /// an archived cron log is a leaked key.
    fn scrub(&self, s: String) -> String {
        // Early return keeps the common (clean) path allocation-free.
        if s.contains(self.token.as_str()) {
            return s.replace(self.token.as_str(), TOKEN_REDACTED);
        }
        s
    }

    /// The model id this client targets (for ledger/report rows).
    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model
    }
    /// Default model — owner pivot 2026-08-28 (second call, same day):
    /// **NO llama** (reads as too old) and **NO glm-4.7-flash** — 4.7 FAILED
    /// the Issue 043 re-designation gate (strict-keep 3/9 in BOTH modes vs
    /// the llama bar 5/9 raw / 6/9 prepass; it beat decline 0/9 and
    /// never-wrong held, but quality sat 40-50% below the bar). glm-5.3-flash
    /// is the designated replacement: the SAME model the dao advisory lane
    /// catalog-verified and live-smoked (its thinking mitigation is the
    /// measured one for GLM-5 — `reasoning_effort:"low"`, probed 2026-08-28:
    /// 3-token "OK" at ~0.37 n vs 48-token reasoning starvation without it;
    /// the GLM-4 `enable_thinking` kwarg does NOT carry over — the dao's
    /// catalog work found the legacy switch 8005-errors on GLM-5, so
    /// [`build_request_body`] routes 5.x to `reasoning_effort` only).
    /// HONEST CAVEAT: 5.3's fixer-QUALITY arms have NOT run (Issue 043
    /// measured 4.7); the re-gate is the standing follow-up, and the 005
    /// model-pivot discipline applies (one-call smoke before any gated use;
    /// no CF creds in CI → the smoke is an ops step). Override without a
    /// rebuild via [`Self::ENV_MODEL`]. qwen3.8-27b stays priced + as the
    /// groq-side free-tier id; it was measured UNUSABLE as the CF fixer
    /// (thinking economics, Bench 049 §qwen).
    pub const DEFAULT_MODEL: &'static str = "@cf/zai-org/glm-5.3-flash";

    /// Env var overriding [`Self::DEFAULT_MODEL`] — precedence: explicit
    /// arg > `RIIR_CF_MODEL` > default (the `GROQ_MODEL` pattern, mirrored
    /// for the CF leg). Empty string is treated as unset.
    pub const ENV_MODEL: &str = "RIIR_CF_MODEL";

    /// Resolve the model id: explicit arg > [`Self::ENV_MODEL`] env >
    /// [`Self::DEFAULT_MODEL`]. Used by [`Self::from_env`] AND the router's
    /// unarmed-CF pricing basis so both honor the same override.
    #[must_use]
    pub fn resolve_model_id(explicit: Option<&str>) -> String {
        let env = std::env::var(Self::ENV_MODEL).ok();
        Self::resolve_model_from(explicit, env.as_deref())
    }

    /// Pure half of [`Self::resolve_model_id`] (testable without touching
    /// the process env — parallel tests never race it).
    fn resolve_model_from(explicit: Option<&str>, env: Option<&str>) -> String {
        explicit
            .filter(|s| !s.is_empty())
            .or_else(|| env.filter(|s| !s.is_empty()))
            .map_or_else(|| Self::DEFAULT_MODEL.to_string(), str::to_string)
    }

    /// Build from `CF_API_TOKEN` + `CF_ACCOUNT_ID` (+ optional model override).
    ///
    /// Issue 046: the model must be in the pricing table (an unpriced model
    /// is refused here, fail-fast, rather than per-call) and the USD daily
    /// cap ledger opens at `KAT_LLM_SPEND_FILE` or `./.heal/kat_llm_spend.json`.
    ///
    /// Override precedence: explicit arg > `RIIR_CF_MODEL` env >
    /// [`Self::DEFAULT_MODEL`] (Issue: owner asks for a no-rebuild model
    /// switch; mirrors `GROQ_MODEL`).
    ///
    /// # Errors
    ///
    /// A human-readable message naming exactly which env var is missing —
    /// never the token value — or the unpriced-model refusal.
    pub fn from_env(model: Option<&str>) -> Result<Self, String> {
        let token = std::env::var("CF_API_TOKEN")
            .map_err(|_| "CF_API_TOKEN is not set — arms A/B need it (export CF_API_TOKEN=...)".to_string())?;
        let account = std::env::var("CF_ACCOUNT_ID").map_err(|_| {
            "CF_ACCOUNT_ID is not set — arms A/B need it (export CF_ACCOUNT_ID=...)".to_string()
        })?;
        let model = Self::resolve_model_id(model);
        crate::llm_spend::price_for(&model).ok_or_else(|| {
            format!(
                "model {model:?} is not in the LLM pricing table — refusing to price a guess \
                 at $0; add it to PRICING_TABLE + bump PRICING_VERSION"
            )
        })?;
        let spend = match std::env::var(crate::llm_spend::ENV_SPEND_FILE) {
            Ok(p) => crate::llm_spend::SpendLedger::open_at(PathBuf::from(p)),
            Err(_) => crate::llm_spend::SpendLedger::open(
                std::env::current_dir().unwrap_or_default().as_path(),
            ),
        };
        Ok(Self {
            agent: ureq::AgentBuilder::new()
                .timeout(CALL_TIMEOUT)
                .timeout_connect(CONNECT_TIMEOUT)
                .build(),
            url: format!("https://api.cloudflare.com/client/v4/accounts/{account}/ai/run/{model}"),
            auth: format!("Bearer {token}"),
            token,
            account,
            model,
            spend,
        })
    }

    /// The USD daily-cap ledger state (Issue 046) — for run summaries and
    /// ledger rows.
    #[must_use]
    pub fn spend_state(&self) -> &crate::llm_spend::SpendState {
        self.spend.state()
    }

    /// One-line cap status for run summaries (cold path).
    #[must_use]
    pub fn spend_summary(&self) -> String {
        self.spend.summary_line()
    }

    /// Server-side usage for THIS model, TODAY (UTC), via the GraphQL
    /// Analytics API — the billing-truth half of the two-meter cost track
    /// (Proposal 005 §cost-tracking; client meter is the other half).
    ///
    /// Returns requests + tokens + **Neurons** (Workers AI's billing unit;
    /// list price $0.011 per 1,000 Neurons). Verified live 2026-08-26: the
    /// wrangler OAuth token's `account:read` scope suffices — no dedicated
    /// analytics token needed. Best-effort by contract: callers treat an
    /// `Err` as "server meter unavailable", never fatal.
    ///
    /// # Errors
    ///
    /// Transport/HTTP/shape errors — never the token.
    #[allow(clippy::many_single_char_names)] // civil-from-days reference notation
    pub fn fetch_usage_today(&self) -> Result<CfUsageToday, String> {
        let today = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() + 86_400) // generous: never split a run at local midnight
            .map(|s| s / 86_400 * 86_400)
            .map_err(|e| format!("clock: {e}"))?;
        // epoch-days -> YYYY-MM-DD (civil-from-days, Howard Hinnant's
        // algorithm — kept verbatim vs the reference; the single-char names
        // ARE the reference notation)
        let days = i64::try_from(today / 86_400).unwrap_or(0);
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        let date = format!("{y:04}-{m:02}-{d:02}");
        let query = format!(
            "{{ viewer {{ accounts(filter: {{accountTag: \"{}\"}}) {{ \
             aiInferenceAdaptiveGroups(limit: 1000, filter: {{date_geq: \"{date}\", date_leq: \"{date}\"}}) {{ \
             dimensions {{ modelId }} \
             sum {{ totalInputTokens totalOutputTokens totalNeurons }} count }} }} }} }}",
            self.account
        );
        let resp = self
            .agent
            .post("https://api.cloudflare.com/client/v4/graphql")
            .set("Authorization", &self.auth)
            .send_json(serde_json::json!({ "query": query }))
            .map_err(|e| self.scrub(format!("usage GraphQL request failed: {e}")))?;
        let v: serde_json::Value = resp
            .into_json()
            .map_err(|e| self.scrub(format!("usage GraphQL response was not JSON: {e}")))?;
        let rows = v
            .pointer("/data/viewer/accounts/0/aiInferenceAdaptiveGroups")
            .and_then(|r| r.as_array())
            .ok_or_else(|| self.scrub(format!("usage GraphQL shape unexpected: {}", clip_payload(&v.to_string()))))?;
        let mut u = CfUsageToday::default();
        for row in rows {
            if row
                .pointer("/dimensions/modelId")
                .and_then(serde_json::Value::as_str)
                != Some(self.model.as_str())
            {
                continue;
            }
            u.requests += row.pointer("/count").and_then(serde_json::Value::as_u64).unwrap_or(0);
            u.input_tokens += row
                .pointer("/sum/totalInputTokens")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            u.output_tokens += row
                .pointer("/sum/totalOutputTokens")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            u.neurons += row
                .pointer("/sum/totalNeurons")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
        }
        Ok(u)
    }
}

/// Server-side per-model usage for one UTC day (the GraphQL analytics row).
#[derive(Debug, Clone, Default)]
pub struct CfUsageToday {
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Workers AI's billing unit (list $0.011 / 1k Neurons).
    pub neurons: f64,
}

/// Build the OpenAI-compat request body, applying per-family thinking
/// mitigations (both measured live — see the `generate` doc for provenance).
/// Pure so the mitigations are unit-testable without a network.
fn build_request_body(
    model: &str,
    prompt: &str,
    max_tokens: usize,
    glm_thinking: bool,
) -> serde_json::Value {
    let content = if model.contains("qwen3") {
        format!("{prompt} /no_think")
    } else {
        prompt.to_string()
    };
    let mut body = serde_json::json!({
        "messages": [{ "role": "user", "content": content }],
        "max_tokens": max_tokens,
        "temperature": 0.0,
    });
    if !glm_thinking {
        if model.contains("glm-5") {
            // GLM-5 generation: `reasoning_effort:"low"` is the honored
            // switch (probed live 2026-08-28 — 3-token "OK" at ~0.37 n vs
            // 48-token reasoning starvation without it). The GLM-4
            // `enable_thinking` kwarg must NOT go here: the legacy switch
            // 8005-errors on GLM-5 (the dao advisory lane's catalog
            // finding, Plan 003).
            body["reasoning_effort"] = serde_json::json!("low");
        } else if model.contains("glm") {
            body["chat_template_kwargs"] =
                serde_json::json!({ "enable_thinking": false });
        }
    }
    body
}

/// Error-prefix classifier for the thinking-model starvation class (see
/// [`parse_chat_result`]). Distinct so harness runs can tell "budget too
/// small" (recoverable, size it) from "unknown schema" (a real bug).
pub const REASONING_OVERFLOW: &str = "reasoning-overflow";

/// Parse the CF `result` object into `(text, reasoning_chars)`.
///
/// Two response schemas ship on Workers AI (measured 2026-08-26):
///   legacy      — `result.response` (String | Array<String>)
///                 e.g. llama-3.3-70b-instruct-fp8-fast
///   chat-style  — `result.choices[0].message.content` + a separate reasoning
///                 field for thinking models (`message.reasoning` for
///                 qwen3.x, `message.reasoning_content` DeepSeek-style for
///                 GLM — which emits BOTH at equal length)
///
/// The 169-request arm-A burn on 2026-08-26 was wasted to learning
/// this: the qwen switch left `response` absent and every extraction
/// returned nothing while completion tokens (thinking) billed.
///
/// Null/absent `content` with `finish_reason == "length"` is classified as
/// [`REASONING_OVERFLOW`] — the thinking-model starvation class where
/// reasoning consumed the whole `max_tokens` budget — so callers size the
/// budget or disable thinking instead of misreading "model unusable"
/// (the exact misread the 2026-08-26 GLM smoke + qwen incident shared).
///
/// # Errors
///
/// Overflow/shape errors; the overflow message starts with
/// [`REASONING_OVERFLOW`] so `starts_with` classifies it without parsing.
fn parse_chat_result(result: &serde_json::Value) -> Result<(String, usize), String> {
    if let Some(r) = result
        .pointer("/choices/0/message/content")
        .and_then(serde_json::Value::as_str)
    {
        let rc = [
            result
                .pointer("/choices/0/message/reasoning")
                .and_then(serde_json::Value::as_str),
            result
                .pointer("/choices/0/message/reasoning_content")
                .and_then(serde_json::Value::as_str),
        ]
        .iter()
        .filter_map(|o| *o)
        .map(str::len)
        .max()
        .unwrap_or(0);
        return Ok((r.to_string(), rc));
    }
    let finish = result
        .pointer("/choices/0/finish_reason")
        .and_then(serde_json::Value::as_str);
    if finish == Some("length") {
        return Err(format!(
            "{REASONING_OVERFLOW}: content null with finish_reason=length — raise max_tokens or disable thinking"
        ));
    }
    match result.get("response") {
        Some(serde_json::Value::String(s)) => Ok((s.clone(), 0)),
        Some(serde_json::Value::Array(items)) => Ok((
            items
                .iter()
                .filter_map(|i| i.as_str())
                .collect::<Vec<_>>()
                .join(""),
            0,
        )),
        _ => Err(format!(
            "CF result had neither chat nor legacy response shape: {result}"
        )),
    }
}

impl CfClient {
    /// One generation. Temperature 0 for reproducibility.
    ///
    /// **Issue 046 — the USD hard cap (reserve-then-reconcile):** the call
    /// is priced at the model's published in/out per-M rates against the
    /// per-UTC-day accumulator and CHECKED BEFORE THE REQUEST; at/over cap
    /// the call hard-declines for the rest of the UTC day
    /// (`Err` starting with "LLM daily cap reached") — a terminal-for-today
    /// verdict the CALLER must honor, never retry. A transport failure
    /// releases the reserve (nothing billed); a response that arrived but
    /// failed to parse spends the FULL reserve (the qwen-incident rule:
    /// tokens were billed even though extraction produced nothing).
    ///
    /// # Errors
    ///
    /// Transport/HTTP/shape errors — body text on HTTP failure (CF error
    /// JSON), never the token — plus the two cap-gate refusals (cap reached,
    /// unknown model).
    pub fn generate(&mut self, prompt: &str, max_tokens: usize) -> Result<CfGen, String> {
        let t0 = std::time::Instant::now();
        let max = u64::try_from(max_tokens).unwrap_or(u64::MAX);
        let reserved = match self.spend.reserve(&self.model, prompt, max) {
            Ok(r) => r,
            Err(e) => {
                // Persist the decline count, then surface the verdict.
                let _ = self.spend.save();
                return Err(e.to_string());
            }
        };
        // Crash-safety: persist the outstanding reserve BEFORE the call — a
        // crash mid-call keeps the reserve counted against today's cap.
        let _ = self.spend.save();
        let mut gen = match self.generate_billed(prompt, max_tokens) {
            Ok(g) => g,
            Err((msg, billed)) => {
                if billed {
                    // A `result` was received — tokens were almost certainly
                    // consumed. Spend the reserve (over-count, never under).
                    self.spend.reconcile(reserved, &self.model, 0, 0, false);
                } else {
                    // The request never completed — nothing billed.
                    self.spend.release(reserved);
                }
                let _ = self.spend.save();
                return Err(msg);
            }
        };
        self.spend.reconcile(
            reserved,
            &self.model,
            gen.prompt_tokens,
            gen.completion_tokens,
            gen.usage_reported,
        );
        let _ = self.spend.save();
        gen.latency_ms = t0.elapsed().as_millis();
        Ok(gen)
    }

    /// The pre-046 generation body (the network half, no cap logic). `Err`
    /// carries `(message, billed?)` where `billed` = a `result` object was
    /// received — used by [`Self::generate`] to decide spend-vs-release.
    ///
    /// Qwen3-family models are THINKING models: unmitigated, the reasoning
    /// phase consumes the entire `max_tokens` budget and `content` arrives
    /// null (`finish_reason` `=` `"length"`) — the 2026-08-26 wasted-run
    /// incident class, re-caught by the standing one-call smoke on
    /// 2026-08-27Z's session. The official Qwen3 soft switch is the literal
    /// suffix `/no_think` in the user turn (probed live: reasoning compresses
    /// to a short preamble and content lands within budget;
    /// `enable_thinking:false` is IGNORED by this serving stack — probed,
    /// reasoning still present). Non-qwen3 models pass through unchanged.
    /// GLM mitigation is generation-dependent (probed — Issue 043 T1 for
    /// 4.7, the 2026-08-28 live probes for 5.3): `glm-5*` takes
    /// `reasoning_effort:"low"` (the ONLY honored switch there — the GLM-4
    /// kwarg 8005-errors), glm-4.x takes
    /// `chat_template_kwargs.enable_thinking:false` (0.155 n vs 4.52 n on
    /// the trivial probe, ~29×). Both are exact inversions of qwen, where
    /// the kwarg is IGNORED and the suffix works; on GLM,
    /// `thinking:{type:disabled}` is ignored too (probed).
    /// `RIIR_CF_GLM_THINKING=1` opts back into budgeted reasoning (for
    /// gates comparing the reasoning mode) — read once per call, never
    /// logged.
    fn generate_billed(&self, prompt: &str, max_tokens: usize) -> Result<CfGen, (String, bool)> {
        let glm_thinking =
            std::env::var("RIIR_CF_GLM_THINKING").as_deref() == Ok("1");
        let body = build_request_body(&self.model, prompt, max_tokens, glm_thinking);
        let resp = self
            .agent
            .post(&self.url)
            .set("Authorization", &self.auth)
            .send_json(body)
            .map_err(|e| (self.scrub(format!("CF request failed: {e}")), false))?;
        let v: serde_json::Value = resp
            .into_json()
            .map_err(|e| (self.scrub(format!("CF response was not JSON: {e}")), false))?;
        let result = v
            .get("result")
            .ok_or_else(|| (self.scrub(format!("CF response had no result: {}", clip_payload(&v.to_string()))), false))?;
        let (text, reasoning_chars) =
            parse_chat_result(result).map_err(|e| (e, true))?;
        let usage = result.get("usage");
        let usage_reported = usage.is_some();
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
            neurons: usage
                .and_then(|u| u.get("neurons"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            reasoning_chars,
            usage_reported,
            latency_ms: 0, // set by the caller (the gate wraps the clock)
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
    /// The durable trajectory store (Issue 049's growing-input source),
    /// best-effort opened at `<repo root>/.heal/trajectories.kv`. `None`
    /// when absent/unopenable — the growing channels then degrade to empty
    /// (arm G ≡ arm B for that sample, honestly recorded by the runner).
    pub store: Option<crate::traj_store::TrajStore>,
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
        let store = crate::traj_store::TrajStore::open(std::path::Path::new(".")).ok().map(|(s, _)| s);
        Ok(Self {
            pipeline,
            corpus,
            matcher,
            verifier: ClippyVerifier::new(),
            store,
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

    /// Issue 049 — the growing-input channels for one module: (channel 1)
    /// top-k shape-similar trajectory outcomes from the durable store
    /// (Certified first, Withdrawn excluded by `retrieve`), and (channel 3)
    /// the per-rule failure-pattern summary over the whole store (top rules
    /// by attempts, success rate, up to 2 representative REJECTED pairs).
    ///
    /// Both channels degrade to empty without a store — the runner records
    /// the degradation per sample (arm G ≡ arm B there).
    pub fn trajectory_context(&self, module: &str, k: usize) -> (Vec<TrajectoryExemplar>, Vec<FailurePattern>) {
        use std::collections::BTreeMap;
        /// (`ok`, `fail`, [(`shape`, `fix`, `fail_count`)] rows) — the per-rule fold.
        type RuleFold = (u32, u32, Vec<(String, String, u32)>);
        let Some(store) = &self.store else {
            return (Vec::new(), Vec::new());
        };
        // Channel 1: shape-similar neighbors across ALL rules (empty lint
        // prefix). Certified first, then Heuristic; deterministic order.
        let mut neighbors = store.retrieve(module, "", k);
        neighbors.sort_by(|a, b| {
            let tier_rank = |t: crate::memory::EvidenceTier| {
                u8::from(!matches!(t, crate::memory::EvidenceTier::Certified))
            };
            tier_rank(a.tier)
                .cmp(&tier_rank(b.tier))
                .then(b.bm25.partial_cmp(&a.bm25).unwrap_or(std::cmp::Ordering::Equal))
        });
        let exemplars: Vec<TrajectoryExemplar> = neighbors
            .iter()
            .map(|n| TrajectoryExemplar {
                lint_key: n.lint_key.clone(),
                shape: n.shape.clone(),
                fix: n.fix.clone(),
                success: n.success_count,
                fail: n.fail_count,
                tier: if matches!(n.tier, crate::memory::EvidenceTier::Certified) { "Certified" } else { "Heuristic" },
            })
            .collect();
        // Channel 3: aggregate every row per rule; report the top rules by
        // attempts (the distribution the improver should calibrate against).
        let mut by_rule: BTreeMap<String, RuleFold> = BTreeMap::new();
        for row in store.dump() {
            if matches!(crate::memory::EvidenceTier::from_u8(row.tier), crate::memory::EvidenceTier::Withdrawn) {
                continue;
            }
            let e = by_rule.entry(row.lint_key.clone()).or_insert((0, 0, Vec::new()));
            e.0 += row.success_count;
            e.1 += row.fail_count;
            if row.fail_count > 0 {
                e.2.push((row.shape.clone(), row.fix.clone(), row.fail_count));
            }
        }
        let mut patterns: Vec<FailurePattern> = by_rule
            .into_iter()
            .filter_map(|(rule, (ok, fail, rejected))| {
                let attempts = ok + fail;
                if attempts == 0 {
                    return None;
                }
                let mut rejected = rejected;
                rejected.sort_by_key(|r| std::cmp::Reverse(r.2));
                Some(FailurePattern {
                    rule,
                    attempts,
                    kept: ok,
                    // Display-only rate on tiny counts (≤ store rows); the
                    // exact integers ride alongside in `kept`/`attempts`.
                    #[allow(clippy::cast_precision_loss)]
                    success_rate: ok as f32 / attempts as f32,
                    representative_failures: rejected
                        .into_iter()
                        .take(2)
                        .map(|(s, f, _)| (s, f))
                        .collect(),
                })
            })
            .collect();
        patterns.sort_by(|a, b| b.attempts.cmp(&a.attempts).then(a.rule.cmp(&b.rule)));
        patterns.truncate(5);
        (exemplars, patterns)
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

    // ── build_request_body / parse_chat_result (Issue 043 T3) ────────────

    #[test]
    fn request_body_appends_no_think_for_qwen3_only() {
        let q = build_request_body("@cf/qwen/qwen3.8-27b", "fix this", 128, false);
        assert_eq!(
            q.pointer("/messages/0/content").and_then(|v| v.as_str()),
            Some("fix this /no_think")
        );
        assert!(q.get("chat_template_kwargs").is_none());
    }

    #[test]
    fn request_body_disables_glm_thinking_by_default() {
        let g = build_request_body("@cf/zai-org/glm-4.7-flash", "fix this", 512, false);
        assert_eq!(
            g.pointer("/messages/0/content").and_then(|v| v.as_str()),
            Some("fix this")
        );
        assert_eq!(
            g.pointer("/chat_template_kwargs/enable_thinking"),
            Some(&serde_json::json!(false))
        );
    }

    #[test]
    fn request_body_glm5_uses_reasoning_effort_low() {
        // GLM-5 takes the DIFFERENT switch: reasoning_effort (the GLM-4
        // enable_thinking kwarg 8005-errors there — dao Plan 003 catalog
        // finding). DEFAULT_MODEL is a glm-5, so this is the production
        // body shape now.
        let g = build_request_body(CfClient::DEFAULT_MODEL, "fix this", 512, false);
        assert_eq!(
            g.pointer("/messages/0/content").and_then(|v| v.as_str()),
            Some("fix this")
        );
        assert_eq!(
            g.pointer("/reasoning_effort").and_then(|v| v.as_str()),
            Some("low")
        );
        assert!(
            g.get("chat_template_kwargs").is_none(),
            "the GLM-4 kwarg must NOT ride a glm-5 request (8005-errors)"
        );
    }

    #[test]
    fn request_body_glm5_thinking_env_override_keeps_reasoning() {
        let g = build_request_body(CfClient::DEFAULT_MODEL, "fix this", 512, true);
        assert!(g.get("reasoning_effort").is_none());
        assert!(g.get("chat_template_kwargs").is_none());
    }

    #[test]
    fn resolve_model_precedence_explicit_over_env_over_default() {
        // Pure core — no process-env races (parallel tests never touch it).
        let d = CfClient::DEFAULT_MODEL;
        assert_eq!(
            CfClient::resolve_model_from(Some("@cf/qwen/qwen3.8-27b"), Some("@cf/zai-org/glm-4.7-flash")),
            "@cf/qwen/qwen3.8-27b",
            "explicit arg wins over env"
        );
        assert_eq!(
            CfClient::resolve_model_from(None, Some("@cf/zai-org/glm-4.7-flash")),
            "@cf/zai-org/glm-4.7-flash",
            "env wins over default"
        );
        assert_eq!(CfClient::resolve_model_from(None, None), d, "default falls through");
        assert_eq!(
            CfClient::resolve_model_from(Some(""), Some("")),
            d,
            "empty strings are unset, not overrides"
        );
    }

    #[test]
    fn request_body_glm_thinking_env_override_keeps_reasoning() {
        let g = build_request_body("@cf/zai-org/glm-4.7-flash", "fix this", 512, true);
        assert!(g.get("chat_template_kwargs").is_none());
    }

    #[test]
    fn request_body_leaves_non_thinking_models_alone() {
        let l = build_request_body(
            "@cf/meta/llama-3.3-70b-instruct-fp8-fast",
            "fix this",
            512,
            false,
        );
        assert_eq!(
            l.pointer("/messages/0/content").and_then(|v| v.as_str()),
            Some("fix this")
        );
        assert!(l.get("chat_template_kwargs").is_none());
    }

    #[test]
    fn parse_reads_reasoning_content_the_glm_shape() {
        let v = serde_json::json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": "fn fixed() {}",
                    "reasoning": "",
                    "reasoning_content": "think think"
                }
            }]
        });
        let (t, rc) = parse_chat_result(&v).unwrap();
        assert_eq!(t, "fn fixed() {}");
        assert_eq!(rc, "think think".len());
    }

    #[test]
    fn parse_prefers_the_longer_reasoning_field() {
        let v = serde_json::json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": "ok",
                    "reasoning": "longer reasoning here",
                    "reasoning_content": "short"
                }
            }]
        });
        let (_, rc) = parse_chat_result(&v).unwrap();
        assert_eq!(rc, "longer reasoning here".len());
    }

    #[test]
    fn parse_classifies_reasoning_overflow_distinctly() {
        let v = serde_json::json!({
            "choices": [{
                "finish_reason": "length",
                "message": { "content": null, "reasoning": "x" }
            }]
        });
        let e = parse_chat_result(&v).unwrap_err();
        assert!(
            e.starts_with(REASONING_OVERFLOW),
            "overflow must carry the classifier prefix, got: {e}"
        );
    }

    #[test]
    fn parse_null_content_with_stop_is_the_generic_shape_error() {
        let v = serde_json::json!({
            "choices": [{
                "finish_reason": "stop",
                "message": { "content": null }
            }]
        });
        let e = parse_chat_result(&v).unwrap_err();
        assert!(!e.starts_with(REASONING_OVERFLOW));
    }

    #[test]
    fn parse_legacy_string_and_array_shapes_still_work() {
        let s = serde_json::json!({ "response": "legacy text" });
        assert_eq!(parse_chat_result(&s).unwrap().0, "legacy text");
        let a = serde_json::json!({ "response": ["a", "b"] });
        assert_eq!(parse_chat_result(&a).unwrap().0, "ab");
    }

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

    // ── Issue 049 — the growing-input prompt ───────────────────────────

    #[test]
    fn prompt_growing_carries_both_channels_and_keeps_baselines_intact() {
        let fs = vec![FewShot { rule_id: "manual_map".into(), buggy: "b".into(), fixed: "f".into() }];
        let traj = vec![TrajectoryExemplar {
            lint_key: "uninlined_format_args".into(),
            shape: "format!(\"{}\", x)".into(),
            fix: "format!(\"{x}\")".into(),
            success: 4,
            fail: 1,
            tier: "Certified",
        }];
        let patterns = vec![FailurePattern {
            rule: "match_bool".into(),
            attempts: 6,
            kept: 3,
            success_rate: 0.5,
            representative_failures: vec![("match b { true => x, false => y }".into(), "if b { x } else { y }".into())],
        }];
        let g = prompt_growing("MOD", Some("CAND"), &fs, &traj, &patterns);
        // Channel 1: prior outcomes block.
        assert!(g.contains("Prior fix attempts on shape-similar code"));
        assert!(g.contains("uninlined_format_args [Certified: 4 ok / 1 failed]"));
        // Channel 3: failure-pattern block.
        assert!(g.contains("Historical fix-reliability by rule"));
        assert!(g.contains("match_bool: 3/6 attempts kept (50%)"));
        assert!(g.contains("REJECTED"));
        // Arm B's content carries through (channel 2 + few-shots).
        assert!(g.contains("CAND"));
        assert!(g.contains("example (manual_map) before"));
        // The closing line is last (spliced, not duplicated).
        assert!(g.ends_with("no explanation."));
        assert_eq!(g.matches("Respond with ONLY").count(), 1);
        // Deterministic.
        assert_eq!(g, prompt_growing("MOD", Some("CAND"), &fs, &traj, &patterns));
        // Empty channels → byte-identical to arm B (the degradation case).
        assert_eq!(prompt_growing("MOD", Some("CAND"), &fs, &[], &[]), prompt_prepass("MOD", Some("CAND"), &fs));
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

    // ── Issue 051 T12: the fallback leg gets the free leg's hardening ──

    #[test]
    fn connect_timeout_is_bounded_well_under_the_call_timeout() {
        assert!(super::CONNECT_TIMEOUT < super::CALL_TIMEOUT);
        assert!(super::CONNECT_TIMEOUT.as_secs() <= 15);
        // A full window must not hang for hours on connects alone.
        assert!(super::CONNECT_TIMEOUT.as_secs() * 25 <= 300);
    }

    #[test]
    fn clip_payload_bounds_a_server_controlled_dump() {
        let small = "{\"error\":\"nope\"}";
        assert_eq!(super::clip_payload(small), small, "short payloads pass through");
        let big = "x".repeat(super::MAX_PAYLOAD_IN_ERROR * 4);
        let out = super::clip_payload(&big);
        assert!(out.len() < big.len(), "must shrink");
        assert!(out.contains("clipped"), "must say so: {out}");
        assert!(out.contains(&big.len().to_string()), "must report the true size");
    }

    #[test]
    fn clip_payload_never_splits_a_utf8_char() {
        // A multi-byte char straddling the cut would panic a naive slice.
        let s = "é".repeat(super::MAX_PAYLOAD_IN_ERROR);
        let out = super::clip_payload(&s);
        assert!(out.contains("clipped"));
        assert!(out.is_char_boundary(0));
    }
}

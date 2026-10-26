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
//!   `CLOUDFLARE_OPS_API_TOKEN` and is NEVER logged or included in any error string.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::behavioural_equivalence::{self, Verdict};
use crate::domain::HealEntry;
use crate::draft::{DomainId, LatentMatcher};
use crate::kat_footer::TOKENS_PER_MTOK;
use crate::l4_daemon::TRAINED_INSTRUCTION;
use crate::pruner::RustAnalyzerPruner;
use crate::self_evolve::HealPipeline;
use crate::ClippyVerifier;

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
        // Re-picked 2026-09-13 (the P10 intake made `vec_init_then_push` a
        // corpus entry, breaking this list's off-corpus premise — the assert's
        // own designed response: "the fixture must be re-picked"). Same vec
        // family, still off-corpus, real clippy lint.
        "clear_with_drain",
        "pub fn f(v: &mut Vec<i32>) { v.drain(..); }",
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
    (
        // Re-picked 2026-09-18 (Batch 155 made `ptr_arg` a corpus entry,
        // breaking this list's off-corpus premise — the assert's own
        // designed response: "the fixture must be re-picked"). Same
        // signature-shape family, still off-corpus, real clippy lint.
        "unused_self",
        "pub struct S;\nimpl S { pub fn f(&self) {} }",
    ),
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
    let Some(start) = text.find("```") else {
        return text;
    };
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
        let raw =
            std::fs::read_to_string(&p).map_err(|e| format!("cannot read {}: {e}", p.display()))?;
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

/// Load the FULL v2edit corpus — Plan 225 T1.1's harvest population: the
/// seven test chunks + `sft_train` + `sft_test` + the indist chunk, deduped by
/// BLAKE3(module) first-seen, in deterministic file/row order (the chunks
/// first — they are the lint-LABELLED slice, so their ids win the dedup).
///
/// Unlike [`load_v2edit`] a row MAY carry no lint label — the harvest's
/// labelling pass (`ClippyVerifier::discover_lints`) targets whatever
/// actually fires in the scaffold. `cap = 0` means uncapped.
///
/// # Errors
///
/// Returns a `String` if a corpus file is missing (refusing a silently
/// truncated population) or a row fails to parse.
pub fn load_v2edit_full(data_dir: &Path, cap: usize) -> Result<Vec<Sample>, String> {
    #[derive(serde::Deserialize)]
    struct Row {
        prompt: String,
        response: String,
        #[serde(default)]
        lint: Option<String>,
        #[serde(default)]
        lints: Option<Vec<String>>,
    }
    const FILES: [&str; 10] = [
        "clippy_l4_v2edit_test_chunk_00.jsonl",
        "clippy_l4_v2edit_test_chunk_01.jsonl",
        "clippy_l4_v2edit_test_chunk_02.jsonl",
        "clippy_l4_v2edit_test_chunk_03.jsonl",
        "clippy_l4_v2edit_test_chunk_04.jsonl",
        "clippy_l4_v2edit_test_chunk_05.jsonl",
        "clippy_l4_v2edit_test_chunk_06.jsonl",
        "clippy_l4_v2edit_sft_train.jsonl",
        "clippy_l4_v2edit_sft_test.jsonl",
        "clippy_l4_v2edit_indist_chunk_00.jsonl",
    ];
    let mut out: Vec<Sample> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for file in FILES {
        let p = data_dir.join(file);
        let raw = std::fs::read_to_string(&p).map_err(|e| {
            format!(
                "cannot read {}: {e} — the harvest population must not be silently truncated",
                p.display()
            )
        })?;
        for (i, line) in raw.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            if cap != 0 && out.len() >= cap {
                return Ok(out);
            }
            let row: Row = serde_json::from_str(line)
                .map_err(|e| format!("{}:{}: bad JSONL row: {e}", p.display(), i + 1))?;
            let module = prompt_module(&row.prompt).to_string();
            if !seen.insert(blake3::hash(module.as_bytes()).to_hex().to_string()) {
                continue; // duplicate module — first sighting owns the case id
            }
            let mut lints = row.lints.unwrap_or_default();
            if let Some(l) = row.lint {
                lints.push(l);
            }
            let stem = file
                .strip_prefix("clippy_l4_v2edit_")
                .unwrap_or(file)
                .trim_end_matches(".jsonl");
            out.push(Sample {
                id: format!("v2edit_all:{stem}:{i:03}"),
                lints,
                module,
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
        matches!(
            self,
            MissClass::OffCorpus | MissClass::ServedNoParse | MissClass::Declined
        )
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
///
/// Byte-contract: arms A/B are reproducible baselines — the Issue-067 split
/// of this fn into header + [`push_prepass_body`] is output-preserving by
/// construction (the body is the same pushes, called from one place).
#[must_use]
pub fn prompt_prepass(module: &str, modelless: Option<&str>, fewshots: &[FewShot]) -> String {
    let mut s = String::new();
    s.push_str(TRAINED_INSTRUCTION);
    s.push_str("\n\n");
    push_prepass_body(&mut s, module, modelless, fewshots);
    s
}

/// The arm-B prompt body AFTER the instruction header (module + candidate +
/// few-shots + the closing respond-line). Extracted so the Issue-067 arm G'
/// builder can insert its conditioning channels BETWEEN the header and the
/// code module without duplicating (and drifting from) this body.
fn push_prepass_body(s: &mut String, module: &str, modelless: Option<&str>, fewshots: &[FewShot]) {
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
    s.push_str(
        "\nRespond with ONLY the corrected lines that differ from the input code, no explanation.",
    );
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
/// The two growing-input channel blocks, rendered exactly as arm G has
/// always rendered them (byte-preserved — arm G stays a reproducible
/// baseline). Shared by arm G (channels spliced before the closing line)
/// and arm G' (channels placed before the code module). Each block starts
/// with a leading `\n` separator; empty channels render to "".
fn render_growing_channels(
    trajectories: &[TrajectoryExemplar],
    failure_summary: &[FailurePattern],
) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    if !trajectories.is_empty() {
        s.push_str("\nPrior fix attempts on shape-similar code (from the trajectory store — outcomes are ground truth, not suggestions):\n");
        for t in trajectories {
            let _ = write!(
                s,
                "--- {} [{}: {} ok / {} failed] ---\n before:\n{}\n after:\n{}",
                t.lint_key,
                t.tier,
                t.success,
                t.fail,
                indent4(&t.shape),
                indent4(&t.fix)
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
    s
}

#[must_use]
pub fn prompt_growing(
    module: &str,
    modelless: Option<&str>,
    fewshots: &[FewShot],
    trajectories: &[TrajectoryExemplar],
    failure_summary: &[FailurePattern],
) -> String {
    let s = prompt_prepass(module, modelless, fewshots);
    // Splice the channels BEFORE the closing respond-line.
    let closing =
        "\nRespond with ONLY the corrected lines that differ from the input code, no explanation.";
    let base = s.strip_suffix(closing).unwrap_or(&s).to_string();
    let mut s = base;
    s.push_str(&render_growing_channels(trajectories, failure_summary));
    s.push_str(closing);
    s
}

/// Issue 067 arm G' prompt (arm `G2` on the `fixer_regate` CLI): the arm-B
/// pre-pass with the SAME-RULE conditioning channels placed BEFORE the code
/// module. Two deltas vs arm G, both from katgpt-rs Research 530
/// (arXiv:2609.02702 "Trace as State"):
///
/// 1. Placement. For causal processors, conditioning placed BEFORE the
///    context dominated the reverse (26/27 model×task×metric cells) — arm G
///    appended the channels after the module (the paper's controlled
///    variable); G' puts them between the instruction header and the module
///    so the model reads same-rule outcomes BEFORE the input they condition
///    (`[z, C]`, not `[C, z]`).
/// 2. Relevance. The channels are same-rule only (the harness's
///    `trajectory_context_same_rule`) — the paper's Random-Trace arm scored
///    BELOW no-trace, so weakly-relevant conditioning content is actively
///    harmful; cross-rule exemplars cannot ride along.
///
/// The channel rendering is arm G's, byte-preserved. Degradation contract:
/// empty channels ⇒ byte-identical to [`prompt_prepass`] (arm B) — pinned
/// by test. Arm A/B prompts are untouched.
#[must_use]
pub fn prompt_growing_same_rule(
    module: &str,
    modelless: Option<&str>,
    fewshots: &[FewShot],
    trajectories: &[TrajectoryExemplar],
    failure_summary: &[FailurePattern],
) -> String {
    let mut s = String::new();
    s.push_str(TRAINED_INSTRUCTION);
    s.push_str("\n\n");
    // Condition-first: channels before the code module.
    s.push_str(&render_growing_channels(trajectories, failure_summary));
    push_prepass_body(&mut s, module, modelless, fewshots);
    s
}

fn indent4(text: &str) -> String {
    text.lines()
        .map(|l| format!("    {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Issue 073 T3: the Bt8 anchor-ledger arm ──────────────────────────────

/// The discrete identity fields of one fix target, rendered as a verbatim
/// anchor ledger (`XBridge` role separation: the DISCRETE anchor channel
/// determines WHICH entities the fixer works on; the module prose is the
/// continuous channel that shapes HOW it reasons). Bench 868 measured the
/// pure-compression alternative (`Bt7`/`Bt7Neg` dense gists) as a LOSS on
/// every axis — the anchor ledger is the complementary arm: keep the prose,
/// restate the identity fields discretely so they survive inattentive
/// reading.
#[derive(Debug, Clone, Default)]
pub struct FixAnchors {
    /// The lint/rule id the fix is filed under (e.g. `ptr_arg`).
    pub rule_id: Option<String>,
    /// The rustc error code when the miss is compile-driven (e.g. `E0599`).
    pub error_code: Option<String>,
    /// Symbol names the fix must preserve/touch, in display order.
    pub symbols: Vec<String>,
    /// The source span, carried as the caller's verbatim string
    /// (line/byte form is the caller's convention).
    pub span: Option<String>,
}

impl FixAnchors {
    /// True when nothing is set — the anchor ledger then renders to "" and
    /// [`prompt_anchor_ledger`] degrades byte-identically to arm B.
    fn is_empty(&self) -> bool {
        self.rule_id.is_none()
            && self.error_code.is_none()
            && self.symbols.is_empty()
            && self.span.is_none()
    }

    /// One anchor per line, verbatim, deterministic order (rule, error,
    /// symbols, span). Empty fields are omitted, never rendered as blanks.
    fn render(&self) -> String {
        use std::fmt::Write as _;
        let mut s = String::new();
        s.push_str("Identity anchors (verbatim — these fields identify the exact fix target):\n");
        if let Some(r) = &self.rule_id {
            let _ = writeln!(s, "  rule: {r}");
        }
        if let Some(e) = &self.error_code {
            let _ = writeln!(s, "  error code: {e}");
        }
        if !self.symbols.is_empty() {
            let _ = write!(s, "  symbols: {}", self.symbols.join(", "));
            s.push('\n');
        }
        if let Some(sp) = &self.span {
            let _ = writeln!(s, "  span: {sp}");
        }
        s
    }
}

/// Issue 073 T3 arm Bt8 prompt: the arm-B pre-pass ([`prompt_prepass`]) + a
/// DISCRETE anchor ledger placed BEFORE the code module (condition-first —
/// the arm-G' placement finding, `[z, C]` not `[C, z]`). The A/B this arms:
/// plain prose baseline (arm B) vs prose + anchor ledger, on the pinned
/// corpus, under the Bench-868 protocol — identity fields crossing only
/// inside prose is the measured failure class; this builder gives the A/B
/// its anchor arm without forking the arm-B body.
///
/// Degradation contract (the house pattern): empty [`FixAnchors`] ⇒
/// byte-identical to [`prompt_prepass`] — pinned by test.
#[must_use]
pub fn prompt_anchor_ledger(
    module: &str,
    modelless: Option<&str>,
    fewshots: &[FewShot],
    anchors: &FixAnchors,
) -> String {
    if anchors.is_empty() {
        return prompt_prepass(module, modelless, fewshots);
    }
    let mut s = String::new();
    s.push_str(TRAINED_INSTRUCTION);
    s.push_str("\n\n");
    // Condition-first: the anchor ledger sits between the instruction header
    // and the code module, matching arm G''s `[z, C]` placement.
    s.push_str(&anchors.render());
    s.push('\n');
    push_prepass_body(&mut s, module, modelless, fewshots);
    s
}

/// Issue 073 T2/T3: same-kind identifier rename at word boundaries —
/// deterministic, no regex dep. Replaces EVERY boundary occurrence of `from`
/// with `to` (a partial rename would break the twin instrument). Shared by
/// the builder pins (tests) and the Bt8 A/B runner (`fixer_regate --arm
/// BT8`) so both exercise ONE implementation.
#[must_use]
pub fn perturb_symbol_ident(code: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(code.len() + 16);
    let mut rest = code;
    while let Some(pos) = rest.find(from) {
        let (before, after) = rest.split_at(pos);
        out.push_str(before);
        rest = after;
        let ident_end = rest[from.len()..]
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .map_or(rest.len(), |i| i + from.len());
        if ident_end == from.len() {
            out.push_str(to);
            rest = &rest[from.len()..];
        } else {
            out.push_str(&rest[..ident_end]);
            rest = &rest[ident_end..];
        }
    }
    out.push_str(rest);
    out
}

/// Issue 073 T3: word-boundary `contains` — the grading predicate for "the
/// healed fix references the swapped symbol". Boundary semantics match
/// [`perturb_symbol_ident`], so the rename target and the original are
/// DISJOINT predicates: `greedy_path` is not found inside `greedy_path_alt`.
#[must_use]
pub fn ident_mentioned(text: &str, ident: &str) -> bool {
    let mut from = 0usize;
    while let Some(rel) = text[from..].find(ident) {
        let at = from + rel;
        let before_ok = at == 0 || !text[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_');
        let after_at = at + ident.len();
        let after_ok = after_at >= text.len()
            || !text[after_at..].starts_with(|c: char| c.is_alphanumeric() || c == '_');
        if before_ok && after_ok {
            return true;
        }
        from = after_at;
    }
    false
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
    depths.shrink_to_fit();
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
    out.shrink_to_fit();
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
        let left_ok = at == 0 || !(bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_');
        let after = at + pat.len();
        let right_ok =
            after >= bytes.len() || !(bytes[after].is_ascii_alphanumeric() || bytes[after] == b'_');
        if !left_ok || !right_ok {
            continue;
        }
        let Some(brace) = src[at..].find('{') else {
            continue;
        };
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
    if n == 0 {
        None
    } else {
        Some(&rest[..n])
    }
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
    /// The generation is a FAITHFUL whole-module rewrite: it parses as a
    /// file and preserves every top-level named item the module declares
    /// (Bench 114). Span-preserving proposer fixes are exactly this shape;
    /// the snippet strategies mangle them into parseable-but-broken modules
    /// (`substitute_fn_spans` splits on blank lines and bails on real
    /// multi-fn bodies, then the LCS line-merge drops mid-item lines). The
    /// item-preservation gate is what makes it v2edit-safe: a corpus
    /// snippet or a content-dropping response fails it and falls through.
    Rewrite,
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
            Self::Verbatim => "verbatim",
            Self::Rewrite => "rewrite",
            Self::FnSpans => "fn-spans",
            Self::LcsBounded => "lcs-bounded",
            Self::LcsUnbounded => "lcs-unbounded",
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
pub fn apply_edit(
    module: &str,
    response: &str,
    verbatim_ok: bool,
) -> Option<(String, MergeStrategy)> {
    let gen = extract_code_block(response);
    let parses = |s: &str| syn::parse_file(s).is_ok();

    if verbatim_ok {
        let t = gen.trim();
        if !t.is_empty() && parses(t) {
            return Some((t.to_string(), MergeStrategy::Verbatim));
        }
    }
    // Faithful whole-module rewrite (Bench 114): tried for EVERY source —
    // the item-preservation gate is the trust, not the caller's flag. A
    // response that keeps every top-level named item cannot be a snippet
    // or a content drop, so taking it whole is safe exactly where the
    // verbatim branch is not.
    if let Some(rewritten) = faithful_rewrite(module, gen) {
        return Some((rewritten, MergeStrategy::Rewrite));
    }
    if let Some(m) = substitute_fn_spans(module, gen) {
        if parses(&m) {
            return Some((m, MergeStrategy::FnSpans));
        }
    }
    let orig: Vec<&str> = module.lines().collect();
    let resp: Vec<&str> = gen.lines().collect();
    for (bounded, strat) in [
        (true, MergeStrategy::LcsBounded),
        (false, MergeStrategy::LcsUnbounded),
    ] {
        if let Some(lines) = lcs_merge(&orig, &resp, bounded) {
            let m = lines.join("\n");
            if parses(&m) {
                return Some((m, strat));
            }
        }
    }
    None
}

/// A faithful whole-module rewrite: the response parses as a Rust file,
/// differs from the module, and declares every top-level named item the
/// module declares (additions allowed, `use` items exempt — a fix may
/// legitimately add or remove imports). Span-preserving proposer fixes and
/// honest full rewrites pass; corpus-shaped snippets and content-dropping
/// responses fail and fall through to the snippet strategies.
#[must_use]
fn faithful_rewrite(module: &str, response: &str) -> Option<String> {
    let t = response.trim();
    if t.is_empty() || t == module.trim() {
        return None;
    }
    let mod_file = syn::parse_file(module).ok()?;
    let resp_file = syn::parse_file(t).ok()?;
    let top_level_names = |f: &syn::File| {
        f.items
            .iter()
            .filter_map(|i| match i {
                syn::Item::Fn(x) => Some(x.sig.ident.to_string()),
                syn::Item::Struct(x) => Some(x.ident.to_string()),
                syn::Item::Enum(x) => Some(x.ident.to_string()),
                syn::Item::Trait(x) => Some(x.ident.to_string()),
                syn::Item::Type(x) => Some(x.ident.to_string()),
                syn::Item::Static(x) => Some(x.ident.to_string()),
                syn::Item::Const(x) => Some(x.ident.to_string()),
                syn::Item::Mod(x) => Some(x.ident.to_string()),
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>()
    };
    let mod_names = top_level_names(&mod_file);
    let resp_names = top_level_names(&resp_file);
    if mod_names.iter().all(|n| resp_names.contains(n)) {
        Some(t.to_string())
    } else {
        None
    }
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
    if let Ok(f) = syn::parse_file(code) {
        v.visit_file(&f);
    } else if let Ok(f) = syn::parse_file(&format!("fn _wrap() {{\n{code}\n}}")) {
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

/// Plan 225 T1.4 clause (c): gold is REFUSED when the candidate ADDS an
/// `allow`/`expect` attribute — lint silencing, not healing. Compares
/// occurrence counts (not presence) so a pre-existing allowance survives a
/// legitimate fix untouched.
#[must_use]
pub fn attr_silence_added(original: &str, merged: &str) -> bool {
    let more = |pat: &str| merged.matches(pat).count() > original.matches(pat).count();
    more("#[allow(") || more("#[expect(") || more("#![allow(") || more("#![expect(")
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
/// take minutes. The leaf's const now (issue 124; same 180 s value, one home).
const CALL_TIMEOUT: Duration = riir_llm::transport::CALL_TIMEOUT;
/// Connect-phase ceiling, load-bearing SEPARATELY from [`CALL_TIMEOUT`]: under a
/// single overall timeout a blackholed TCP connect is indistinguishable from a
/// slow model, so it stalls for the full 3 min — × every span in the window.
/// Connecting is fast or broken; nothing in between. The leaf's const now
/// (issue 124; same 10 s value, one home).
const CONNECT_TIMEOUT: Duration = riir_llm::transport::CONNECT_TIMEOUT;
/// Redaction placeholder, should the token ever surface in a payload.
const TOKEN_REDACTED: &str = "<CLOUDFLARE_OPS_API_TOKEN redacted>";
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
/// Issue 124: the chat-generation wire half (URL shape, body builder,
/// response parse, failure classification, bearer scrub on classified
/// errors) is the [`riir_llm`] leaf; this client keeps the env/pricing
/// resolution, the USD cap ledger, the GraphQL usage meter (which the
/// leaf does not speak), and the `billed?` accounting the cap's
/// reserve-then-reconcile needs.
pub struct CfClient {
    /// Shared across both request paths (chat POST via the leaf's
    /// [`riir_llm::transport::post_chat_on`] + the GraphQL usage read).
    agent: ureq::Agent,
    /// Everything before the route — the leaf's `Wire::Cf::url` appends
    /// `/ai/run/{model}` (the URL shape lives in ONE place now; the old
    /// full-URL-in-a-field spelling is what the dao smoke caught
    /// drifting elsewhere).
    base: String,
    /// The pre-formatted `Bearer <token>` header value, built ONCE.
    ///
    /// Perf: the two request paths no longer allocate a header String per
    /// call. Security: the secret occupies ONE heap location for the
    /// client's lifetime instead of being copied into a fresh allocation per
    /// request and dropped un-zeroed. Mirrors `groq::OaiClient` (Issue 051
    /// T10) — same defect class, same fix, every leg of the ladder.
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
        // Deliberately no token, no base (base embeds the account id).
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

    /// Build from `CLOUDFLARE_OPS_API_TOKEN` + `CLOUDFLARE_ACCOUNT_ID` (+ optional model override).
    ///
    /// Issue 046: the model must be in the pricing table (an unpriced model
    /// is refused here, fail-fast, rather than per-call) and the USD daily
    /// cap ledger opens at `KAT_LLM_SPEND_FILE` or `./.refine/kat_llm_spend.json`.
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
        let token = std::env::var("CLOUDFLARE_OPS_API_TOKEN").map_err(|_| {
            "CLOUDFLARE_OPS_API_TOKEN is not set — arms A/B need it (export CLOUDFLARE_OPS_API_TOKEN=...)"
                .to_string()
        })?;
        let account = std::env::var("CLOUDFLARE_ACCOUNT_ID").map_err(|_| {
            "CLOUDFLARE_ACCOUNT_ID is not set — arms A/B need it (export CLOUDFLARE_ACCOUNT_ID=...)".to_string()
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
            base: format!("https://api.cloudflare.com/client/v4/accounts/{account}"),
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
            .ok_or_else(|| {
                self.scrub(format!(
                    "usage GraphQL shape unexpected: {}",
                    clip_payload(&v.to_string())
                ))
            })?;
        let mut u = CfUsageToday::default();
        for row in rows {
            if row
                .pointer("/dimensions/modelId")
                .and_then(serde_json::Value::as_str)
                != Some(self.model.as_str())
            {
                continue;
            }
            u.requests += row
                .pointer("/count")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
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

/// Error-prefix classifier for the thinking-model starvation class — ONE
/// home now (the leaf; issue 124). This alias and `groq::REASONING_OVERFLOW`
/// re-export the same const, so the old both-features drift-pin test
/// retired with the duplication. The leaf's [`riir_llm::parse_cf_result`]
/// emits it.
pub const REASONING_OVERFLOW: &str = riir_llm::REASONING_OVERFLOW;

// Issue 124: `build_request_body` and `parse_chat_result` moved to the
// leaf as `chat_body_for` + `parse_cf_result` (tests moved with them) —
// one home for the measured thinking switches and the both-schema result
// walk. The overflow-message wording is byte-identical (pinned by leaf
// tests + kat_drain's `starts_with(REASONING_OVERFLOW)` escalation).
//
// The measured facts that justified the moved code, kept here for the
// archaeology: the 169-request arm-A burn on 2026-08-26 (the qwen switch
// left `response` absent, every extraction returned nothing, completion
// tokens billed anyway); GLM emits BOTH `reasoning` and
// `reasoning_content` at equal length (qwen emits only `reasoning`);
// and the GLM-5.3-flash budget floor — max_tokens 32 starves, 64 is
// marginal (1-in-N), >= 128 returns finish:stop + a 3-token answer every
// time, so the harness default `max_new = 128` sits exactly on it.

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
    /// logged. All of those switches live in the leaf's body builder now
    /// (issue 124); this call site just picks the mode.
    fn generate_billed(&self, prompt: &str, max_tokens: usize) -> Result<CfGen, (String, bool)> {
        let glm_thinking = std::env::var("RIIR_CF_GLM_THINKING").as_deref() == Ok("1");
        let body = riir_llm::chat_body_for(
            riir_llm::Wire::Cf,
            &self.model,
            prompt,
            u64::try_from(max_tokens).unwrap_or(u64::MAX),
            glm_thinking,
        );
        // The POST rides the leaf transport (auth header, both timeouts,
        // failure classification, bearer scrub). `billed = false`: the
        // request never completed or was refused at the HTTP layer —
        // nothing to reconcile (the reserve releases).
        let leg = self.wire_leg();
        let raw = riir_llm::transport::post_chat_on(&self.agent, &leg, &body)
            .map_err(|f| (f.message().to_string(), false))?;
        let v: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| (self.scrub(format!("CF response was not JSON: {e}")), false))?;
        let result = v.get("result").ok_or_else(|| {
            (
                self.scrub(format!(
                    "CF response had no result: {}",
                    clip_payload(&v.to_string())
                )),
                false,
            )
        })?;
        // A `result` object existed — the schema walk is the leaf's
        // [`riir_llm::parse_cf_result`]; a failure HERE is billed (the
        // qwen-incident rule: tokens were consumed even if extraction
        // produced nothing).
        let (text, reasoning_chars) = riir_llm::parse_cf_result(result).map_err(|e| (e, true))?;
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

    /// The leg view for the shared [`riir_llm`] transport — the CF wire:
    /// the model rides the URL path (the leaf appends `/ai/run/{model}`),
    /// the body is model-free.
    fn wire_leg(&self) -> riir_llm::Leg {
        riir_llm::Leg {
            label: "cf".to_string(),
            base: self.base.clone(),
            bearer: self.token.clone(),
            model: self.model.clone(),
            wire: riir_llm::Wire::Cf,
        }
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
            Verdict::Equivalent { cases } => Self::Equivalent { cases: *cases },
            Verdict::Divergent { input, .. } => Self::Divergent {
                input: input.clone(),
            },
            Verdict::Unsupported { reason } => Self::Unsupported {
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
    /// The MERGED module compiled under the oracle (Plan 225 T1.4 clause
    /// (a)). Distinct from `oracle_ran`: a compile failure still "ran", and
    /// its empty lint list must not read as a findings reduction.
    pub after_compiled: bool,
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
    let control_fires = before.ran
        && before
            .present_lints
            .iter()
            .any(|l| l == &format!("clippy::{}", sample.lints[0]));

    let (merged, strategy) = match apply_edit(
        &sample.module,
        generation,
        sample.source == SampleSource::Reachable,
    ) {
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
        after_compiled: false,
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
    s.after_compiled = after.compiled;
    s.findings_after = after.present_lints.len();
    // `after.compiled` gates both reductions: a module that failed to
    // compile reports an empty lint list (errors, not clippy codes), which
    // before the gate read as "strictly fewer + target resolved" for code
    // that never linted at all (Plan 225 T1.4 clause (a)).
    s.strictly_fewer = s.oracle_ran && s.after_compiled && s.findings_after < s.findings_before;
    s.target_resolved = s.oracle_ran
        && s.after_compiled
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
/// One [`RegateHarness::classify_outcome`] result — the miss class, the
/// modelless serve, its confidence score, and the L0–L3 candidate pool
/// (the Plan 202 R6 arm-H escalation input).
pub struct ClassifyOutcome {
    /// The miss classification (unchanged semantics).
    pub miss: MissClass,
    /// The modelless fix `heal` served (`None` = declined).
    pub served: Option<String>,
    /// The modelless confidence (`evolution_score`, 0.0 when declined) —
    /// the manifest row's `modelless_cap` worthiness axis.
    pub score: f32,
    /// The candidate codes in candidate order, deduped — the rerank arm's
    /// presentation (the seat picks among THESE by index).
    pub pool: Vec<String>,
}

pub struct RegateHarness {
    pub pipeline: HealPipeline,
    pub corpus: ClippyCorpus,
    pub matcher: LatentMatcher,
    pub verifier: ClippyVerifier,
    /// The durable trajectory store (Issue 049's growing-input source),
    /// best-effort opened at `<repo root>/.refine/trajectories.kv`. `None`
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
        // Store path is CWD-relative by default (the arm-G convention — run
        // from the repo whose dev store you want), overridable per run so an
        // A/B can point at a scratch/planted store without CWD tricks.
        let store_dir = std::env::var("RIIR_REGATE_STORE_DIR").unwrap_or_else(|_| ".".to_string());
        let store = crate::traj_store::TrajStore::open(std::path::Path::new(&store_dir))
            .ok()
            .map(|(s, _)| s);
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
        let outcome = self.classify_outcome(lint, module);
        (outcome.miss, outcome.served)
    }

    /// Plan 202 R6 arm H — [`Self::classify`] plus the modelless confidence
    /// score and the L0–L3 candidate pool: the full escalation-chain input
    /// (worthiness gate reads `score`, the seat rerank reads `pool`).
    /// Served-score semantics: a declined case scores `0.0` (a total miss
    /// is maximally escalation-eligible — `modelless_cap > 0` routes it).
    pub fn classify_outcome(&mut self, lint: &str, module: &str) -> ClassifyOutcome {
        let candidates: Vec<crate::ClippyFixEntry> = self.corpus.lookup(lint).cloned().collect();
        let in_corpus = !candidates.is_empty();
        let outcome = self.pipeline.heal_with_pool(lint, module, &candidates);
        let (miss, served, score) = if let Some(r) = outcome.result {
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
            (class, Some(r.fixed_code), r.evolution_score)
        } else {
            let class = if in_corpus {
                MissClass::Declined
            } else {
                MissClass::OffCorpus
            };
            (class, None, 0.0)
        };
        ClassifyOutcome {
            miss,
            served,
            score,
            pool: outcome.pool,
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
    pub fn trajectory_context(
        &self,
        module: &str,
        k: usize,
    ) -> (Vec<TrajectoryExemplar>, Vec<FailurePattern>) {
        match &self.store {
            Some(store) => all_rule_context(store, module, k),
            None => (Vec::new(), Vec::new()),
        }
    }

    /// Issue 067 T2 — the SAME-RULE channels for one sample: both channels
    /// restricted to the queried lint's family. Channel 1 retrieves with the
    /// boundary-safe `{lint}:` key prefix (the `TrajStore` row-key boundary —
    /// `traj:{lint_key}:{shape_hash}` — which also admits the rustc
    /// `{code}:{strategy}` rows under one code); channel 3 folds only that
    /// family's rows. This is the relevance fix: arm G's cross-rule channels
    /// are the paper's Random-Trace class — weakly-relevant conditioning
    /// that measured BELOW no-trace (katgpt-rs Research 530).
    ///
    /// Both channels degrade to empty without a store or with no same-rule
    /// rows — arm G' ≡ arm B there, recorded per-row by the runner.
    pub fn trajectory_context_same_rule(
        &self,
        module: &str,
        lint_key: &str,
        k: usize,
    ) -> (Vec<TrajectoryExemplar>, Vec<FailurePattern>) {
        match &self.store {
            Some(store) => same_rule_context(store, module, lint_key, k),
            None => (Vec::new(), Vec::new()),
        }
    }
}

/// Arm G's channel build over a store: shape-similar neighbors across ALL
/// rules (empty lint prefix) + the whole-store failure-pattern summary.
fn all_rule_context(
    store: &crate::traj_store::TrajStore,
    module: &str,
    k: usize,
) -> (Vec<TrajectoryExemplar>, Vec<FailurePattern>) {
    let exemplars = growing_exemplars(store, module, "", k);
    let patterns = fold_failure_patterns(store.dump(), None);
    (exemplars, patterns)
}

/// Arm G''s channel build over a store: both channels restricted to the
/// queried lint's family. The retrieval prefix is the boundary-safe
/// `{lint}:` (never a bare prefix — `ptr_arg:` must not admit a
/// hypothetical `ptr_arg_extra` row); the fold matches the row's own
/// `lint_key` field with the same boundary rule (the field carries no
/// shape-hash suffix, so it needs the `== fam || starts_with(fam + ":")`
/// form rather than the key prefix).
fn same_rule_context(
    store: &crate::traj_store::TrajStore,
    module: &str,
    lint_key: &str,
    k: usize,
) -> (Vec<TrajectoryExemplar>, Vec<FailurePattern>) {
    let exemplars = growing_exemplars(store, module, &format!("{lint_key}:"), k);
    let patterns = fold_failure_patterns(store.dump(), Some(lint_key));
    (exemplars, patterns)
}

/// Channel 1: top-k shape-similar neighbors (`lint_prefix` = "" for all
/// rules, `{lint}:` for one family), Certified first then Heuristic,
/// deterministic order (arm G's original comparator, extracted).
fn growing_exemplars(
    store: &crate::traj_store::TrajStore,
    module: &str,
    lint_prefix: &str,
    k: usize,
) -> Vec<TrajectoryExemplar> {
    let mut neighbors = store.retrieve(module, lint_prefix, k);
    neighbors.sort_by(|a, b| {
        let tier_rank = |t: crate::memory::EvidenceTier| {
            u8::from(!matches!(t, crate::memory::EvidenceTier::Certified))
        };
        tier_rank(a.tier)
            .cmp(&tier_rank(b.tier))
            .then(b.bm25.total_cmp(&a.bm25))
    });
    neighbors
        .iter()
        .map(|n| TrajectoryExemplar {
            lint_key: n.lint_key.clone(),
            shape: n.shape.clone(),
            fix: n.fix.clone(),
            success: n.success_count,
            fail: n.fail_count,
            tier: if matches!(n.tier, crate::memory::EvidenceTier::Certified) {
                "Certified"
            } else {
                "Heuristic"
            },
        })
        .collect()
}

/// Channel 3: the per-rule failure-pattern fold (`family` = `None` for arm
/// G's whole-store summary, `Some(lint)` for arm G''s same-rule
/// restriction). Withdrawn rows are excluded (the T7.4 rule); rules with
/// zero attempts are dropped; top rules by attempts (the distribution the
/// improver should calibrate against), up to 2 representative REJECTED
/// pairs each, 5 rules max.
fn fold_failure_patterns(
    rows: Vec<crate::traj_store::TrajRowDoc>,
    family: Option<&str>,
) -> Vec<FailurePattern> {
    use std::collections::BTreeMap;
    /// (`ok`, `fail`, [(`shape`, `fix`, `fail_count`)] rows) — the per-rule fold.
    type RuleFold = (u32, u32, Vec<(String, String, u32)>);
    let mut by_rule: BTreeMap<String, RuleFold> = BTreeMap::new();
    for row in rows {
        if matches!(
            crate::memory::EvidenceTier::from_u8(row.tier),
            crate::memory::EvidenceTier::Withdrawn
        ) {
            continue;
        }
        if let Some(fam) = family {
            let in_family = row.lint_key == fam
                || row
                    .lint_key
                    .strip_prefix(fam)
                    .is_some_and(|rest| rest.starts_with(':'));
            if !in_family {
                continue;
            }
        }
        let e = by_rule
            .entry(row.lint_key.clone())
            .or_insert((0, 0, Vec::new()));
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
    patterns
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
    pub fn add(
        &mut self,
        s: &CaseScore,
        reachable: bool,
        prompt_tokens: u64,
        completion_tokens: u64,
        price_in_per_m: f64,
        price_out_per_m: f64,
    ) {
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
            let holds =
                s.contract_ok && !matches!(s.behavioural, Some(BehavSummary::Divergent { .. }));
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
        // f64 precision loss is irrelevant at token counts (< 2^32). The
        // divisor is the TOKEN million (per-Mtok rates) — never
        // MICRO_PER_KAT (riir-dapps Plan 035 T10).
        #[allow(clippy::cast_precision_loss)]
        let cost = prompt_tokens as f64 / TOKENS_PER_MTOK as f64 * price_in_per_m
            + completion_tokens as f64 / TOKENS_PER_MTOK as f64 * price_out_per_m;
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
            if beats {
                "BEATS decline"
            } else {
                "DOES NOT beat decline"
            }
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
#[path = "cf_workers_ai_tests.rs"]
mod tests;

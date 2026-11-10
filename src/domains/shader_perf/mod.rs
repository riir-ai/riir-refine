//! The `shader_perf` domain — WGSL render-pipeline bounded-fix healing
//! (Proposal 014, Plan 134).
//!
//! Seventh retrieval `HealDomain` (after clippy/perf/kernel/sec/docker/dist;
//! `rustc_errors` is a driver-based surface, not a retrieval domain). Scope:
//! **`@vertex`/`@fragment` render-pipeline WGSL source patterns with bounded
//! fix-spaces** — texture-sample budgets, loop boundedness, stage traffic,
//! branch-vs-sample divergence, derivative safety. `@compute` entry points
//! stay [`crate::domains::kernel_opt`] territory (the routing split is the
//! entry-point attribute set — mechanical, no heuristic; measured 2026-09-16:
//! 0 file-level mixed shaders in-tree, but the classifier handles the
//! fft-ocean class anyway).
//!
//! ## Why this domain exists (Proposal 014)
//!
//! Source-level WGSL perf analysis is an empty field: `tint`/`naga` validate
//! semantics and stop; spirv-opt fixes at the IR where the source author
//! never sees it; frame profilers measure after the fact. The consumers are
//! real and named — the upcoming **shader-toy editor** (owner-stated
//! 2026-09-16, repo placement TBD; its submit/save gate is the natural
//! promotion pull) and riir-dapps Proposal 002's Stage-2 mint gate
//! ("`naga` validation + texture/loop caps" — the caps half was prose until
//! this domain).
//!
//! ## Precision-first stance (the `wgsl_scan` precedent, third no-`syn`
//! ## scanner after docker and dist)
//!
//! Zero new dependencies by default: the scanner is a hand-written
//! block-aware line scanner (no `syn`, no `regex` — the docker discipline).
//! Every finding cites a corpus `rule_id`. Patterns are restricted to shapes
//! that are **textually unambiguous**; where a rule's real discriminator
//! needs dataflow the text does not carry, the pattern is conservative
//! (biased to miss, never to fire on clean code) or recorded as
//! not-line-expressible in v1. The negative-control gate
//! (`tests/shader_perf_gates.rs`) pins ZERO findings over the vgpu port
//! corpus — a real false positive on a hero-verified port demotes or kills
//! the rule (the garbage-in guard).
//!
//! ## The refusals are load-bearing (the DL3059 lesson)
//!
//! `branch-divergent-sample-select` is a DECISION rule, never a one-sided
//! rewrite — hoisting a sample out of a branch changes implicit-derivative
//! semantics and always-sampling costs bandwidth on the not-taken path.
//! [`refuses`] names the shapes no proposer may emit, and
//! `tests/shader_perf_gates.rs` pins them.

use serde::{Deserialize, Serialize};

use crate::domain::{HealDomain, HealEntry};

pub mod corpus;
pub mod proposers;
pub mod scanner;

// Tier-0 (text invariants) ships with the domain; the naga Tier-1 half is
// cfg-gated INSIDE on `shader_verify` (dep:naga).
pub mod verify;

/// The render-pipeline stage(s) a WGSL source declares entry points for.
///
/// This is the routing classifier (Proposal 014 §"The routing split"):
/// `@compute` → `kernel_opt`, `@vertex`/`@fragment` → this domain. A file
/// may declare several (the fft-ocean class: compute sim + fragment shade);
/// each domain's scanner fires only inside its own stage's functions, so
/// mixed files are scanned by both with findings routed by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StageSet {
    /// Any `@compute` entry point present.
    pub compute: bool,
    /// Any `@vertex` entry point present.
    pub vertex: bool,
    /// Any `@fragment` entry point present.
    pub fragment: bool,
}

impl StageSet {
    /// Does this source have any render-pipeline entry point (this domain's
    /// territory)?
    #[must_use]
    pub fn has_render(self) -> bool {
        self.vertex || self.fragment
    }
}

/// Classify a WGSL source by its entry-point attributes.
///
/// Comment-stripped, string-literal-masked line scan for the three stage
/// attributes. A `// @fragment` doc mention is not a stage (the
/// `attention_batched.wgsl` T2-smoke lesson from the kernel scanner).
#[must_use]
pub fn entry_stages(source: &str) -> StageSet {
    let mut set = StageSet::default();
    for raw in source.lines() {
        let masked = mask_string_literals(&strip_line_comment(raw));
        if masked.contains("@compute") {
            set.compute = true;
        }
        if masked.contains("@vertex") {
            set.vertex = true;
        }
        if masked.contains("@fragment") {
            set.fragment = true;
        }
    }
    set
}

/// Strip a trailing `//` line comment (the kernel-scanner limitation
/// carries over unchanged: a `//` inside a string literal is pathological in
/// a shader file and accepted as a limitation — but see
/// [`mask_string_literals`], which the STAGE classifier applies before
/// testing).
pub(crate) fn strip_line_comment(line: &str) -> String {
    match line.find("//") {
        Some(idx) => line[..idx].to_string(),
        None => line.to_string(),
    }
}

/// Replace string-literal contents with spaces so attribute/token scans
/// cannot fire on quoted text (a `"@fragment"` in an error string is not a
/// stage; a `//`-like sequence inside a literal is not a comment either —
/// this mask is the cheap half of that, applied where it matters).
pub(crate) fn mask_string_literals(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_string = false;
    let mut prev_backslash = false;
    for ch in line.chars() {
        if in_string {
            if ch == '"' && !prev_backslash {
                in_string = false;
                out.push('"');
            } else {
                out.push(' ');
            }
            prev_backslash = ch == '\\' && !prev_backslash;
        } else if ch == '"' {
            in_string = true;
            prev_backslash = false;
            out.push('"');
        } else {
            out.push(ch);
        }
    }
    out
}

/// The rule family (parallel to `DockerFamily`/`SecFamily`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShaderFamily {
    /// Texture-sample budgets (implicit-LOD hazards, per-invocation counts).
    SampleBudget,
    /// Loop boundedness in render stages (the TDR class; the mint gate's
    /// "loop caps" verbatim).
    LoopBoundedness,
    /// Vertex↔fragment stage traffic (hoistable invariants, vertex fetches).
    StageTraffic,
    /// Branch-vs-sample divergence decisions.
    BranchCoherence,
    /// Derivative/uniformity safety (`dpdx`/`fwidth` placement).
    DerivativeSafety,
}

impl ShaderFamily {
    /// Stable label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::SampleBudget => "sample-budget",
            Self::LoopBoundedness => "loop-boundedness",
            Self::StageTraffic => "stage-traffic",
            Self::BranchCoherence => "branch-coherence",
            Self::DerivativeSafety => "derivative-safety",
        }
    }
}

/// One `shader_perf` rule: buggy/fixed fixture pair + family (mirrors
/// `SecRule`/`DockerRule`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderRule {
    /// The rule id (kebab-case, the `rust_perf` convention).
    pub rule_id: String,
    /// The render-pipeline concept in one line.
    pub shader_concept: String,
    /// The buggy code that triggers the rule.
    pub buggy_code: String,
    /// The fixed code that resolves the rule.
    pub fixed_code: String,
    /// The family.
    pub family: ShaderFamily,
}

impl HealEntry for ShaderRule {
    #[inline]
    fn rule_id(&self) -> &str {
        &self.rule_id
    }

    #[inline]
    fn buggy_code(&self) -> &str {
        &self.buggy_code
    }

    #[inline]
    fn fixed_code(&self) -> &str {
        &self.fixed_code
    }
}

/// The Tier-0 verdict (always available, no deps): structural invariants a
/// bounded fix must preserve, plus the `naga` tier when `shader_verify` is on.
///
/// `entry_points_intact` + `bindings_unchanged` are the Proposal 014 Tier-0
/// contract: a render-pipeline fix may NOT add/remove entry points or move
/// a `@group(N) @binding(M)` number — those are the pipeline-layout contract
/// with the host, and a byte-identical binding multiset is textually
/// checkable.
#[derive(Debug, Clone)]
pub struct ShaderVerdict {
    /// Whether the proposed fix re-parses under the Tier-0 structural pass
    /// (balanced braces + at least one render entry point).
    pub parses: bool,
    /// The `@stage` entry-point multiset is unchanged pre/post.
    pub entry_points_intact: bool,
    /// The `@group(N) @binding(M)` multiset is unchanged pre/post.
    pub bindings_unchanged: bool,
    /// The `naga` validation tier (Tier 1): `Some(true)` = module parses +
    /// validates post-fix; `Some(false)` = `naga` rejected it; `None` = the
    /// tier is off/absent (degraded to Tier-0 with ONE log line — the
    /// Issue 018 oracle-split: "could not check" ≠ "check failed").
    pub naga_agrees: Option<bool>,
}

/// The `shader_perf` healing domain — `HealDomain` impl.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShaderPerfDomain;

impl HealDomain for ShaderPerfDomain {
    type Entry = ShaderRule;
    type Verdict = ShaderVerdict;

    fn build_corpus() -> Vec<Self::Entry> {
        corpus::build_shader_seed_corpus().entries
    }

    fn validate(rule_id: &str, proposed_fix: &str) -> Self::Verdict {
        let buggy = corpus::build_shader_seed_corpus()
            .entries
            .into_iter()
            .find(|r| r.rule_id == rule_id)
            .map(|r| r.buggy_code)
            .unwrap_or_default();
        verify::tier0_verdict(&buggy, proposed_fix)
    }
}

/// The anti-cargo-cult refusal table (the DL3059 lesson): shapes no proposer
/// in this domain may EVER emit, regardless of how the input is shaped.
///
/// These are named and tested, not documented-and-forgotten — the docker
/// `DL3059` precedent (consolidating consecutive `RUN`s fights `BuildKit` caching)
/// is the exact class: incumbent tooling folklore that is anti-modern HERE.
#[must_use]
pub fn refuses() -> &'static [&'static str] {
    &[
        // A one-sided branch-divergent rewrite: hoisting textureSample out
        // of a branch (or pushing it into both arms) changes implicit-
        // derivative semantics and trades divergence for bandwidth on the
        // not-taken path. The rule ANNOTATES the decision; the AUTHOR makes
        // it. (branch-divergent-sample-select ships suggest-only forever.)
        "one-sided-branch-sample-rewrite",
        // textureSample → textureSampleLevel with a GUESSED LOD: the LOD is
        // only reconstructible where the sampling is screen-space-uniform
        // (the fullscreen-quad class). A proposer that substitutes a
        // constant LOD without that evidence is a visual-semantics break
        // wearing a perf fix's clothes. The bounded proposer fires ONLY on
        // the loop-carried shape AND leaves the LOD term reviewable.
        "guessed-lod-substitution",
        // Loop-guard counters that wrap: u32 overflow on the guard makes
        // the "bounded" loop unbounded again at 2^32 iterations. The guard
        // must break BEFORE the wrap, and the proposer's does.
        "wrapping-guard-counter",
    ]
}

/// Convenience re-export for the scanner surface (the count-pin needle
/// convention: the corpus uses the positional `rule()` shorthand like
/// docker/dist, so `    rule(` is the dashboard count needle — the
/// `CORPUS_DOMAIN_TABLE` convention; a constructor-plumbing needle counts 2
/// at every rev and lies about the corpus, Issue 066).
pub use scanner::{ShaderFinding, ShaderScanner, entry_point_multiset, group_binding_multiset};

// Plan 134 T2.1: the frame-time budget tier — this domain's measurement
// VERDICT half (the rules are here; the sidecars are riir-shader's, issue
// 018 there). The module lives UNGATED at the crate root
// (`crate::frame_time_report` — the corpus_coverage/horizon_report law:
// pure over its inputs, lib-tested at default features); this gated
// re-export is the domain-charter handle, so domain consumers reach it at
// `domains::shader_perf::frame_time`.

#[cfg(feature = "shader_perf")]
pub use crate::frame_time_report as frame_time;

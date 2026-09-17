//! The block-aware line scanner for the `shader_perf` domain (Plan 134
//! T0.2). No `syn`, no `regex` — the docker discipline, third hand-written
//! scanner in the tree.
//!
//! ## How it reads WGSL
//!
//! A single pass over comment-stripped, string-masked lines with a brace
//! stack. Blocks are classified at OPEN time by the keyword that introduces
//! them:
//!
//! - `fn` carrying a pending `@vertex`/`@fragment` attribute opens a
//!   **render-stage function** block (the domain's territory); a bare helper
//!   `fn` opens an unattributed block whose contents are conservatively
//!   skipped (stage unknown); `@compute` fns are `kernel_opt` territory.
//! - `loop` / `while` / `for` open **loop** blocks (`while (true)` its own
//!   kind); `if` / `else` open **branch** blocks.
//!
//! Per-line accounting is sequential: a leading `}` closes one block (the
//! `} else {` chain), the keyword opener pushes its block, content flags
//! accumulate into the RIGHT ancestors (a `break` bounds only the nearest
//! enclosing loop; samples/derivatives mark every open loop/branch), and
//! the line's remaining `}`s close blocks — so one-liner bodies
//! (`if (x) { break; }`) evaluate correctly.
//!
//! ## The v1 pattern table
//!
//! | rule (family) | textual tell | class |
//! |---|---|---|
//! | `unbounded-render-loop-guard` (`LoopBoundedness`) | a `loop`/`while(true)` block inside a render-stage fn containing ZERO exit tokens (`break`/`return` — `discard` does not count, WGSL demotes it) — provably infinite at the text level (the TDR class; the mint gate's "loop caps" verbatim) | **bounded** (guard insertion) |
//! | `loop-carried-sample-to-explicit-lod` (`SampleBudget`) | `textureSample(` inside a `loop`/`while` block inside a `@fragment` fn — implicit derivatives under potentially non-uniform trip counts; TWO-REGIME (explicit LOD only where reconstructible), so the finding carries the annotation | suggest-only v1 |
//! | `branch-divergent-sample-select` (`BranchCoherence`) | an `if` block followed by its `else` where EXACTLY ONE arm contains `textureSample(` — the divergence-vs-bandwidth decision, annotated, never rewritten (the DL3059 discipline) | suggest-only forever |
//! | `const-fold-loop-bound` (`LoopBoundedness`) | a render-stage loop whose condition references a field of a `var<uniform>` — the bound is constant for the pipeline's lifetime; specialize-or-not is a permutation-cost decision | suggest-only |
//! | `fragment-sample-budget-annotation` (`SampleBudget`) | a `@fragment` fn with > [`MEASURED_PORT_FETCH_MAX`] texture-fetch calls (>= [`FRAGMENT_FETCH_BUDGET`]) — the EXCESS class above what hero-verified ports carry (measured max 9: three-tsl/surface) | suggest-only (Class B) |
//! | `vertex-stage-texture-fetch-annotation` (`StageTraffic`) | any texture fetch inside a `@vertex` fn — vertex texture fetch cost note (`textureSample` itself is invalid in vertex; `naga` owns that error) | suggest-only (Class B) |
//! | `derivative-in-nonuniform-flow` (`DerivativeSafety`) | a derivative intrinsic inside a loop or branch in a `@fragment` fn — `naga`'s uniformity analysis catches the genuinely non-uniform shapes; this advisory fires pre-render on the rest | suggest-only (Class B) |
//!
//! Conservative misses are acceptable (a real buggy file widens a pattern);
//! a false positive on a clean port poisons the negative-control gate, so
//! patterns stay narrow. Not-line-expressible in v1:
//! `invariant-recompute-to-varying` (needs dataflow from vertex outputs) —
//! corpus entry only, no scanner pattern.

use std::collections::BTreeMap;

use super::{mask_string_literals, strip_line_comment};

/// A finding: one rule citation at one source line. Suggest-only output
/// (the kernel `WgslFinding` shape; the bounded loop-guard proposer is the
/// only --fix surface, driven from its own module).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderFinding {
    /// The corpus rule this finding fronts (the citation contract).
    pub rule_id: &'static str,
    /// 1-based line number in the scanned source.
    pub line: usize,
    /// The matched line, trimmed (the human-review excerpt).
    pub excerpt: String,
}

/// Fragment fetch budget for the Class B annotation. MEASURED on the
/// hero-verified port corpus at Plan-134 landing: the max fetches-per-
/// fragment-fn observed is **9** (three-tsl/surface.wgsl); flare-composite
/// carries 7 legitimately. The annotation therefore fires at 10 or more
/// (the EXCESS class above what verified ports use; the first calibration
/// at 5 or more fired on 4 clean ports, and the negative control demoted
/// it). Mobile bandwidth class: every fetch is a memory round trip per
/// pixel.
pub const FRAGMENT_FETCH_BUDGET: usize = 10;

/// The measured corpus max the budget is floored above (three-tsl/surface
/// at 9; the negative-control record).
pub const MEASURED_PORT_FETCH_MAX: usize = 9;

/// The `@stage` entry-point multiset — for the Tier-0 verdict's
/// `entry_points_intact` (a fix may not add/remove entry points).
#[must_use]
pub fn entry_point_multiset(source: &str) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    let mut pending: Option<&str> = None;
    for raw in source.lines() {
        let masked = mask_string_literals(&strip_line_comment(raw));
        for stage in ["@compute", "@vertex", "@fragment"] {
            if masked.contains(stage) && pending.is_none() {
                pending = Some(stage);
            }
        }
        if pending.is_some() && opens_fn(&masked) {
            if let Some(stage) = pending {
                *out.entry(stage.to_string()).or_insert(0) += 1;
            }
            pending = None;
        }
    }
    out
}

/// The `@group(N) @binding(M)` multiset — for the Tier-0 verdict's
/// `bindings_unchanged` (a fix may not move the pipeline layout). The two
/// attributes may sit on separate lines; a pending group pairs with the
/// next binding (conservative nearest-pairing — a bounded fix does not
/// touch these lines at all, so pairing precision only needs to be stable).
#[must_use]
pub fn group_binding_multiset(source: &str) -> BTreeMap<(u32, u32), usize> {
    let mut out = BTreeMap::new();
    let mut pending_group: Option<u32> = None;
    for raw in source.lines() {
        let masked = mask_string_literals(&strip_line_comment(raw));
        let g = find_attr_u32(&masked, "@group");
        let b = find_attr_u32(&masked, "@binding");
        match (g, b) {
            (Some(g), Some(b)) => *out.entry((g, b)).or_insert(0) += 1,
            (Some(g), None) => pending_group = Some(g),
            (None, Some(b)) => {
                if let Some(g) = pending_group.take() {
                    *out.entry((g, b)).or_insert(0) += 1;
                }
            }
            (None, None) => {}
        }
    }
    out
}

/// `@attr(N)` — the attribute's numeric argument, if parseable as u32.
fn find_attr_u32(line: &str, attr: &str) -> Option<u32> {
    let idx = line.find(attr)?;
    let rest = &line[idx + attr.len()..];
    let open = rest.find('(')?;
    let close = rest[open..].find(')')?;
    rest[open + 1..open + close].trim().parse().ok()
}

/// The stage of a render function being scanned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// `@vertex` entry point (or a fn attributed to it).
    Vertex,
    /// `@fragment` entry point.
    Fragment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Fn,
    Loop,
    WhileTrue,
    For,
    If,
    Else,
    Other,
}

#[derive(Debug, Clone, Copy)]
struct Block {
    kind: BlockKind,
    /// The stage attributed to a Fn block (None for helpers/Other).
    stage: Option<Stage>,
    /// 1-based line where the block opened — the finding line for
    /// loop/branch rules.
    open_line: usize,
    /// `textureSample(` (implicit-LOD sampling only) inside this block.
    has_sample: bool,
    /// An exit token (`break` OR `return`) inside — either bounds the
    /// loop's runtime (`return` exits the whole fn, which ends the loop
    /// too; `discard` does NOT — WGSL demotes it and execution continues).
    /// Accumulated into the NEAREST enclosing loop only (an exit exits the
    /// innermost loop).
    has_exit: bool,
    /// Texture-fetch calls inside (any of the four intrinsics) — for the
    /// per-fn budget annotations.
    fetch_count: usize,
}

impl Block {
    fn is_loop(&self) -> bool {
        matches!(self.kind, BlockKind::Loop | BlockKind::WhileTrue | BlockKind::For)
    }
    fn is_branch(&self) -> bool {
        matches!(self.kind, BlockKind::If | BlockKind::Else)
    }
}

/// The `shader_perf` surface scanner. Construct once, scan many files.
/// Deterministic: output sorted by `(line, rule_id)`, deduped.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShaderScanner;

impl ShaderScanner {
    /// Scan one WGSL source. Render-stage findings only — `@compute`
    /// functions and unattributed helpers are skipped by construction.
    /// One sequential ladder (112 lines): the brace-stack + pending-stage
    /// invariants live in ONE place — splitting it would spread them.
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn scan(&self, source: &str) -> Vec<ShaderFinding> {
        let mut findings: Vec<ShaderFinding> = Vec::new();
        let mut stack: Vec<Block> = Vec::new();
        let mut pending_stage: Option<Stage> = None;
        // The most recently closed If block: `(open_line, has_sample)`. An
        // `else` opener consumes it; anything else that opens first lets it
        // go stale (an else can only follow the if's `}` directly).
        let mut last_closed_if: Option<(usize, bool)> = None;
        let mut uniform_vars: Vec<String> = Vec::new();

        for (i, raw) in source.lines().enumerate() {
            let lineno = i + 1;
            let line = mask_string_literals(&strip_line_comment(raw));
            let trimmed = line.trim();

            // File-scope uniform var declarations (outside any fn).
            if stack.iter().all(|b| b.kind != BlockKind::Fn) {
                if let Some(name) = uniform_var_name(&line) {
                    uniform_vars.push(name);
                }
            }

            if line.contains("@vertex") {
                pending_stage = Some(Stage::Vertex);
            }
            if line.contains("@fragment") {
                pending_stage = Some(Stage::Fragment);
            }

            // 1. Leading `}` closes one block (the `} else {` chain).
            if trimmed.starts_with('}') {
                if let Some(closed) = stack.pop() {
                    let paired_if = Self::note_close(&mut last_closed_if, &closed);
                    Self::dispatch_close(&mut findings, closed, &stack, paired_if);
                }
            }

            // 2. The keyword opener this line introduces, if any.
            let opener = classify_opener(&line, pending_stage);
            if let Some((kind, stage)) = opener {
                if kind == BlockKind::Fn {
                    pending_stage = None;
                }
                if kind == BlockKind::Else {
                    // The pairing completes at ELSE CLOSE (the else's own
                    // sample flag is only known then) — see on_else_close.
                }
                stack.push(Block {
                    kind,
                    stage,
                    open_line: lineno,
                    has_sample: false,
                    has_exit: false,
                    fetch_count: 0,
                });
                // A non-Else opener invalidates any pending if/else pair.
                if kind != BlockKind::Else {
                    last_closed_if = None;
                }
            } else if line.contains('{') && !trimmed.starts_with('}') {
                stack.push(Block {
                    kind: BlockKind::Other,
                    stage: None,
                    open_line: lineno,
                    has_sample: false,
                    has_exit: false,
                    fetch_count: 0,
                });
                last_closed_if = None;
            }

            // 3. Accumulate content flags into the right ancestors. The
            // enclosing render stage is the nearest Fn block's stage.
            let fn_stage = stack
                .iter()
                .rev()
                .find(|b| b.kind == BlockKind::Fn)
                .and_then(|b| b.stage);
            let has_exit_tok = contains_word(&line, "break") || contains_word(&line, "return");
            let has_sample_tok = line.contains("textureSample(");
            let has_fetch_tok = has_sample_tok
                || line.contains("textureSampleLevel(")
                || line.contains("textureSampleCompare(")
                || line.contains("textureLoad(");
            let has_deriv_tok = line.contains("dpdx(")
                || line.contains("dpdy(")
                || line.contains("fwidth(")
                || line.contains("ddx(")
                || line.contains("ddy(");

            if has_exit_tok {
                if let Some(b) = stack.iter_mut().rev().find(|b| b.is_loop()) {
                    b.has_exit = true;
                }
            }
            if has_sample_tok {
                for b in stack.iter_mut().rev() {
                    if b.is_loop() || b.is_branch() {
                        b.has_sample = true;
                    }
                }
            }
            if has_fetch_tok {
                if let Some(b) = stack.iter_mut().rev().find(|b| b.kind == BlockKind::Fn) {
                    b.fetch_count += 1;
                }
                // Vertex-stage per-fn annotation is emitted at fn close off
                // `fetch_count` (the aggregate is the honest signal — a fn
                // with any fetch is the cost note).
            }

            // 4. Per-line rules (extracted — the scan walk stays
            // readable; see per_line_rules for the rule table).
            per_line_rules(
                &mut findings,
                &stack,
                fn_stage,
                &uniform_vars,
                &line,
                lineno,
                has_deriv_tok,
            );

            // 5. The line's remaining `}`s close blocks (one-liner bodies,
            // `}}` ends). The opener's own brace was already pushed.
            let total_closes = line.matches('}').count();
            let leading_close = usize::from(trimmed.starts_with('}'));
            let mut remaining = total_closes.saturating_sub(leading_close);
            while remaining > 0 {
                remaining -= 1;
                if let Some(closed) = stack.pop() {
                    let paired_if = Self::note_close(&mut last_closed_if, &closed);
                    Self::dispatch_close(&mut findings, closed, &stack, paired_if);
                }
            }
        }
        findings.sort_by(|a, b| (a.line, a.rule_id).cmp(&(b.line, b.rule_id)));
        findings.dedup();
        findings.shrink_to_fit();
        findings
    }

    /// Record a block close in the if/else pairing state; returns the
    /// pending `(if_line, if_has_sample)` when the CLOSED block is an Else
    /// (its pairing partner), else None.
    fn note_close(
        last_closed_if: &mut Option<(usize, bool)>,
        closed: &Block,
    ) -> Option<(usize, bool)> {
        if closed.kind == BlockKind::If {
            *last_closed_if = Some((closed.open_line, closed.has_sample));
            None
        } else if closed.kind == BlockKind::Else {
            last_closed_if.take()
        } else {
            None
        }
    }

    /// The unified close dispatch (both the leading-`}` and trailing-`}`
    /// paths — the else-pairing must run on EITHER).
    fn dispatch_close(
        findings: &mut Vec<ShaderFinding>,
        closed: Block,
        stack: &[Block],
        paired_if: Option<(usize, bool)>,
    ) {
        if closed.kind == BlockKind::Else {
            Self::on_else_close(findings, &closed, paired_if);
            return;
        }
        Self::on_close(findings, closed, stack);
    }

    /// Close-time evaluation for a block.
    fn on_close(findings: &mut Vec<ShaderFinding>, closed: Block, stack: &[Block]) {
        // Stage resolution: the closed Fn carries its own; anything else
        // inherits the nearest STILL-OPEN Fn ancestor's stage (the loop's
        // stage is the fn it sits in — the stage lives on the ancestor,
        // not the loop block).
        let stage = match closed.kind {
            BlockKind::Fn => closed.stage,
            _ => stack
                .iter()
                .rev()
                .find(|b| b.kind == BlockKind::Fn)
                .and_then(|b| b.stage),
        };
        let Some(stage) = stage else {
            return; // helper fns / Other blocks: stage unknown, skipped
        };
        match closed.kind {
            BlockKind::Fn => {
                // Per-fn aggregate annotations (Class B).
                if stage == Stage::Fragment && closed.fetch_count >= FRAGMENT_FETCH_BUDGET {
                    findings.push(ShaderFinding {
                        rule_id: "fragment-sample-budget-annotation",
                        line: closed.open_line,
                        excerpt: format!(
                            "@fragment fn with {} texture fetches (mobile bandwidth class)",
                            closed.fetch_count
                        ),
                    });
                }
                if stage == Stage::Vertex && closed.fetch_count > 0 {
                    findings.push(ShaderFinding {
                        rule_id: "vertex-stage-texture-fetch-annotation",
                        line: closed.open_line,
                        excerpt: format!(
                            "@vertex fn with {} texture fetch(es) — vertex fetch cost class",
                            closed.fetch_count
                        ),
                    });
                }
            }
            BlockKind::Loop | BlockKind::WhileTrue => {
                if !closed.has_exit {
                    findings.push(ShaderFinding {
                        rule_id: "unbounded-render-loop-guard",
                        line: closed.open_line,
                        excerpt: if closed.kind == BlockKind::WhileTrue {
                            "while(true) with no break — provably unbounded (TDR class)".to_string()
                        } else {
                            "loop with no break — provably unbounded (TDR class)".to_string()
                        },
                    });
                }
                if stage == Stage::Fragment && closed.has_sample {
                    findings.push(ShaderFinding {
                        rule_id: "loop-carried-sample-to-explicit-lod",
                        line: closed.open_line,
                        excerpt: "textureSample inside a potentially-unbounded loop — \
                            implicit derivatives are implementation-defined here \
                            (two-regime: explicit LOD only where reconstructible)"
                            .to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    /// The if/else pairing decision (`branch-divergent-sample-select`):
    /// fires only where EXACTLY one arm samples — the annotated decision,
    /// never a rewrite (the DL3059 refusal).
    fn on_else_close(
        findings: &mut Vec<ShaderFinding>,
        closed: &Block,
        last_closed_if: Option<(usize, bool)>,
    ) {
        let Some((if_line, if_sample)) = last_closed_if else {
            return;
        };
        if if_sample != closed.has_sample {
            findings.push(ShaderFinding {
                rule_id: "branch-divergent-sample-select",
                line: if_line,
                excerpt: "if/else with sampling on exactly one arm — divergence-vs-bandwidth \
                    decision (annotated; never auto-rewritten)"
                    .to_string(),
            });
        }
    }
}

/// `fn name(` — the `fn` keyword opening a function (attribute lines like
/// The per-line rules (scan step 4). Header-only by design for
/// `const-fold-loop-bound`: the first calibration caught body lines firing
/// through call parens (`sampleClamped(uv - u.params.xy * tap.x)` in
/// flare-blur) — the rule is about the BOUND, which lives on the loop
/// HEADER line.
#[allow(clippy::too_many_arguments)]
fn per_line_rules(
    findings: &mut Vec<ShaderFinding>,
    stack: &[Block],
    fn_stage: Option<Stage>,
    uniform_vars: &[String],
    line: &str,
    lineno: usize,
    has_deriv_tok: bool,
) {
    // const-fold-loop-bound: a loop HEADER line (for/while — the keyword
    // is ON this line) whose condition references a uniform var field.
    if !uniform_vars.is_empty()
        && fn_stage.is_some()
        && (contains_word(line, "for") || contains_word(line, "while"))
    {
        if let Some(cond) = loop_condition_text(line) {
            if uniform_vars.iter().any(|u| cond.contains(&format!("{u}."))) {
                findings.push(ShaderFinding {
                    rule_id: "const-fold-loop-bound",
                    line: lineno,
                    excerpt: line.trim().to_string(),
                });
            }
        }
    }
    // derivative-in-nonuniform-flow: fragment-stage derivative inside a
    // loop or branch.
    if has_deriv_tok && fn_stage == Some(Stage::Fragment) && stack.iter().any(|b| b.is_loop() || b.is_branch())
    {
        findings.push(ShaderFinding {
            rule_id: "derivative-in-nonuniform-flow",
            line: lineno,
            excerpt: line.trim().to_string(),
        });
    }
}

/// Classify the block this line opens: `(kind, stage-when-Fn)`.
fn classify_opener(line: &str, pending_stage: Option<Stage>) -> Option<(BlockKind, Option<Stage>)> {
    // Order matters: `else` before `if` (an `else if` chain opens as an
    // Else — conservative for the pairing rule, which only fires on the
    // plain if/else shape).
    if opens_fn(line) {
        return Some((BlockKind::Fn, pending_stage));
    }
    if contains_word(line, "else") && line.contains('{') {
        return Some((BlockKind::Else, None));
    }
    if contains_word(line, "while") {
        if let Some(cond) = loop_condition_text(line) {
            if cond.replace(' ', "") == "true" {
                return Some((BlockKind::WhileTrue, None));
            }
        }
        return Some((BlockKind::Loop, None));
    }
    if contains_word(line, "loop") && line.contains('{') {
        return Some((BlockKind::Loop, None));
    }
    if contains_word(line, "for") && line.contains('(') && line.contains('{') {
        return Some((BlockKind::For, None));
    }
    if contains_word(line, "if") && line.contains('(') && line.contains('{') {
        return Some((BlockKind::If, None));
    }
    None
}

/// `fn name(` — the `fn` keyword opening a function (attribute lines like
/// `@fragment` alone do not match; `foo(...)` calls do not either).
fn opens_fn(line: &str) -> bool {
    if let Some(idx) = line.find('(') {
        if line[..idx].split_whitespace().any(|t| t == "fn") {
            return true;
        }
    }
    // `fn f<T>(...)` generics: token `fn` still precedes the first `(`.
    contains_word(line, "fn") && line.contains('{') && !line.contains('=')
}

/// `var<uniform> name` — the declared variable's name (file scope).
fn uniform_var_name(line: &str) -> Option<String> {
    let idx = line.find("var<uniform>")?;
    let rest = line[idx + "var<uniform>".len()..].trim_start();
    if rest.starts_with('>') {
        // `var<uniform, ...>`? Not valid WGSL — conservative skip.
        return None;
    }
    let name = rest.split([':', '=', ';']).next()?.trim();
    if name.is_empty() || name.contains('(') || name.contains('<') {
        return None;
    }
    Some(name.to_string())
}

/// Word-boundary `contains` for WGSL keywords (`loop` must not match
/// `find_loop`; `if` must not match `endif` in masked text).
fn contains_word(line: &str, word: &str) -> bool {
    let bytes = line.as_bytes();
    let w = word.as_bytes();
    let mut i = 0;
    while i + w.len() <= bytes.len() {
        if &bytes[i..i + w.len()] == w {
            let before_ok = i == 0 || !is_ident(bytes[i - 1]);
            let after = i + w.len();
            let after_ok = after >= bytes.len() || !is_ident(bytes[after]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Loop/while/for condition text: the first paren group on the line (the
/// `for` header's middle segment is checked as-is — a uniform reference
/// anywhere in the header is the tell; conservative but precise enough).
fn loop_condition_text(line: &str) -> Option<String> {
    let open = line.find('(')?;
    let bytes = line.as_bytes();
    let mut depth_paren = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth_paren += 1,
            b')' => {
                depth_paren -= 1;
                if depth_paren == 0 {
                    return Some(line[open + 1..i].to_string());
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule_ids(src: &str) -> Vec<&'static str> {
        ShaderScanner.scan(src).into_iter().map(|f| f.rule_id).collect()
    }

    #[test]
    fn unbounded_loop_in_fragment_fires() {
        let src = "@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n  loop {\n    let x = 1.0;\n  }\n  return vec4<f32>(0.0);\n}\n";
        let rules = rule_ids(src);
        assert!(rules.contains(&"unbounded-render-loop-guard"), "{rules:?}");
    }

    #[test]
    fn while_true_without_break_fires() {
        let src = "@vertex\nfn vs_main() -> @builtin(position) vec4<f32> {\n  while (true) {\n    let x = 1.0;\n  }\n  return vec4<f32>(0.0);\n}\n";
        let rules = rule_ids(src);
        assert!(rules.contains(&"unbounded-render-loop-guard"), "{rules:?}");
    }

    #[test]
    fn bounded_loop_with_return_exit_is_clean() {
        let src = "@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n  loop {\n    if (u() > 0.5) { return vec4<f32>(1.0); }\n  }\n  return vec4<f32>(0.0);\n}\nfn u() -> f32 { return 0.1; }\n";
        assert_eq!(rule_ids(src), Vec::<&'static str>::new());
    }

    #[test]
    fn bounded_loop_with_break_is_clean() {
        let src = "@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n  loop {\n    if (u() > 0.5) { break; }\n  }\n  return vec4<f32>(0.0);\n}\nfn u() -> f32 { return 0.1; }\n";
        assert_eq!(rule_ids(src), Vec::<&'static str>::new(), "bounded + no sample");
    }

    #[test]
    fn unbounded_loop_in_compute_does_not_fire() {
        let src = "@compute @workgroup_size(64)\nfn cs_main() {\n  loop { let x = 1.0; }\n}\n";
        assert_eq!(rule_ids(src), Vec::<&'static str>::new());
    }

    #[test]
    fn unbounded_loop_in_helper_fn_does_not_fire() {
        let src = "fn helper() -> f32 {\n  loop { let x = 1.0; }\n}\n@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n  return vec4<f32>(helper());\n}\n";
        assert_eq!(rule_ids(src), Vec::<&'static str>::new());
    }

    #[test]
    fn loop_carried_sample_in_fragment_fires() {
        let src = "@fragment\nfn fs_main(t: texture_2d<f32>, s: sampler) -> @location(0) vec4<f32> {\n  loop {\n    if (u() > 0.5) { break; }\n    let c = textureSample(t, s, vec2<f32>(0.5));\n  }\n  return vec4<f32>(0.0);\n}\nfn u() -> f32 { return 0.1; }\n";
        let rules = rule_ids(src);
        assert!(rules.contains(&"loop-carried-sample-to-explicit-lod"), "{rules:?}");
        assert!(!rules.contains(&"unbounded-render-loop-guard"));
    }

    #[test]
    fn sample_in_fragment_no_loop_is_clean() {
        let src = "@fragment\nfn fs_main(t: texture_2d<f32>, s: sampler) -> @location(0) vec4<f32> {\n  let c = textureSample(t, s, vec2<f32>(0.5));\n  return c;\n}\n";
        assert_eq!(rule_ids(src), Vec::<&'static str>::new());
    }

    #[test]
    fn branch_divergent_sample_pair_fires() {
        let src = "@fragment\nfn fs_main(t: texture_2d<f32>, s: sampler) -> @location(0) vec4<f32> {\n  var c = vec4<f32>(0.0);\n  if (u() > 0.5) {\n    c = textureSample(t, s, vec2<f32>(0.5));\n  } else {\n    c = vec4<f32>(1.0);\n  }\n  return c;\n}\nfn u() -> f32 { return 0.1; }\n";
        let rules = rule_ids(src);
        assert!(rules.contains(&"branch-divergent-sample-select"), "{rules:?}");
    }

    #[test]
    fn branch_sample_both_arms_is_clean() {
        let src = "@fragment\nfn fs_main(t: texture_2d<f32>, s: sampler) -> @location(0) vec4<f32> {\n  var c = vec4<f32>(0.0);\n  if (u() > 0.5) {\n    c = textureSample(t, s, vec2<f32>(0.5));\n  } else {\n    c = textureSample(t, s, vec2<f32>(0.25));\n  }\n  return c;\n}\nfn u() -> f32 { return 0.1; }\n";
        assert_eq!(rule_ids(src), Vec::<&'static str>::new());
    }

    #[test]
    fn derivative_in_fragment_loop_fires() {
        let src = "@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n  for (var i = 0; i < 4; i++) {\n    let d = fwidth(d0());\n  }\n  return vec4<f32>(0.0);\n}\nfn d0() -> f32 { return 1.0; }\n";
        let rules = rule_ids(src);
        assert!(rules.contains(&"derivative-in-nonuniform-flow"), "{rules:?}");
    }

    #[test]
    fn derivative_top_level_fragment_is_clean() {
        let src = "@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n  let d = fwidth(1.0);\n  return vec4<f32>(d);\n}\n";
        assert_eq!(rule_ids(src), Vec::<&'static str>::new());
    }

    #[test]
    fn derivative_in_vertex_is_clean() {
        let src = "@vertex\nfn vs_main() -> @builtin(position) vec4<f32> {\n  for (var i = 0; i < 4; i++) {\n    let d = fwidth(1.0);\n  }\n  return vec4<f32>(0.0);\n}\n";
        assert_eq!(rule_ids(src), Vec::<&'static str>::new());
    }

    #[test]
    fn const_fold_loop_bound_fires_on_uniform_bound() {
        let src = "var<uniform> params : Params;\n@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n  for (var i = 0; i < params.steps; i++) {\n    let a = 1.0;\n  }\n  return vec4<f32>(0.0);\n}\n";
        let rules = rule_ids(src);
        assert!(rules.contains(&"const-fold-loop-bound"), "{rules:?}");
    }

    #[test]
    fn const_fold_not_on_literal_bound() {
        let src = "@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n  for (var i = 0; i < 16; i++) {\n    let a = 1.0;\n  }\n  return vec4<f32>(0.0);\n}\n";
        assert_eq!(rule_ids(src), Vec::<&'static str>::new());
    }

    #[test]
    fn fragment_sample_budget_annotates_at_threshold() {
        use std::fmt::Write as _;
        let mut src = String::from(
            "@fragment\nfn fs_main(t: texture_2d<f32>, s: sampler) -> @location(0) vec4<f32> {\n",
        );
        for i in 0..FRAGMENT_FETCH_BUDGET {
            let _ = writeln!(src, "  let c{i} = textureSample(t, s, vec2<f32>(0.5));");
        }
        src.push_str("  return c0;\n}\n");
        let rules = rule_ids(&src);
        assert!(rules.contains(&"fragment-sample-budget-annotation"), "{rules:?}");
    }

    #[test]
    fn fragment_sample_budget_below_threshold_is_clean() {
        let src = "@fragment\nfn fs_main(t: texture_2d<f32>, s: sampler) -> @location(0) vec4<f32> {\n  let c = textureSample(t, s, vec2<f32>(0.5));\n  return c;\n}\n";
        assert_eq!(rule_ids(src), Vec::<&'static str>::new());
    }

    #[test]
    fn vertex_texture_fetch_annotates() {
        let src = "@vertex\nfn vs_main() -> @builtin(position) vec4<f32> {\n  let c = textureLoad(t, s, vec2<i32>(0));\n  return vec4<f32>(0.0);\n}\n";
        let rules = rule_ids(src);
        assert!(rules.contains(&"vertex-stage-texture-fetch-annotation"), "{rules:?}");
    }

    #[test]
    fn scan_is_deterministic_and_sorted() {
        let src = "@fragment\nfn fs_main(t: texture_2d<f32>, s: sampler) -> @location(0) vec4<f32> {\n  loop {\n    if (u() > 0.5) { break; }\n    let c = textureSample(t, s, vec2<f32>(0.5));\n  }\n  return vec4<f32>(0.0);\n}\nfn u() -> f32 { return 0.1; }\n";
        let a = ShaderScanner.scan(src);
        let b = ShaderScanner.scan(src);
        assert_eq!(a, b);
        assert!(a.windows(2).all(|w| (w[0].line, w[0].rule_id) <= (w[1].line, w[1].rule_id)));
    }

    #[test]
    fn entry_stage_classification_and_masking() {
        assert!(super::super::entry_stages("@fragment\nfn f() {}\n").fragment);
        assert!(super::super::entry_stages("@compute\nfn f() {}\n").compute);
        assert!(!super::super::entry_stages("// @fragment doc\nfn f() {}\n").fragment);
        assert!(
            !super::super::entry_stages("let s = \"@fragment in a string\";\nfn f() {}\n").fragment
        );
    }

    #[test]
    fn multiset_helpers() {
        let src = "@group(0) @binding(0) var<uniform> a: A;\n@group(0)\n@binding(1)\nvar<uniform> b: B;\n";
        let ms = group_binding_multiset(src);
        assert_eq!(ms.get(&(0, 0)), Some(&1));
        assert_eq!(ms.get(&(0, 1)), Some(&1));
        let eps = entry_point_multiset("@fragment\nfn f() {}\n@fragment\nfn g() {}\n");
        assert_eq!(eps.get("@fragment"), Some(&2));
    }
}

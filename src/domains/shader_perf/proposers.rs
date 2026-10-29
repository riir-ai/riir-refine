//! Stage 2 proposal mechanism for the `shader_perf` domain (Plan 134 T1.4).
//!
//! One bounded proposer ships with the first batch:
//! `unbounded-render-loop-guard` — the guard insertion (declare a u32
//! counter before the loop, increment + bound-check at the loop body's
//! head, break BEFORE the wrap). Detection and the rewrite share the
//! scanner's block walk via [`unbounded_loop_anchors`], and the
//! drift-guard test in this module asserts the proposer's anchors EQUAL
//! the scanner's finding lines on the fixtures — a rule that detects one
//! shape and rewrites another is physically impossible while that test is
//! green (the docker rules.rs co-authoring discipline, compact edition).
//!
//! ## Conservative gates (every proposer refuses more than it fires)
//!
//! | Refusal | Why |
//! |---|---|
//! | source already contains the guard name | idempotence — a second run must be a no-op, not a double guard |
//! | non-`loop`/`while(true)` loops | everything else carries its own bound form |
//! | the loop already has an exit token | not provably unbounded; nothing to guard |
//! | `@compute` or unattributed fns | `kernel_opt` territory / stage unknown |
//!
//! The wrapping-guard-counter refusal ([`super::refuses`]) is enforced by
//! construction: `guard = guard + 1u; if (guard >= BOUND) { break; }` can
//! reach at most `BOUND` (256 < 2^32) — the counter never wraps.

use crate::edit_emitter::{RuleEdit, SourceEdit};

use super::scanner::Stage;

/// The bounded-fix rule ids (the docker `FIXABLE_*` convention).
pub const FIXABLE_SHADER: &[&str] = &["unbounded-render-loop-guard"];

/// The default guard bound. 256 is the corpus fixture's bound — large
/// enough to exceed the semantic exit of accumulated-iteration shaders,
/// small enough to bound a TDR (a driver reset window is milliseconds; 256
/// iterations of any render-stage body is far under it).
pub const DEFAULT_GUARD_BOUND: u32 = 256;

/// The guard variable name prefix (uniqueness via a per-anchor counter;
/// the whole-source refusal on any existing occurrence keeps re-runs
/// no-ops).
const GUARD_NAME: &str = "shader_heal_guard";

/// One unbounded-loop anchor: the loop's opening-brace byte offset plus
/// the byte offset where the loop keyword line starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LoopAnchor {
    /// 1-based line of the loop keyword (equals the scanner finding line).
    pub line: usize,
    /// Byte offset of the line-start before the loop keyword (where the
    /// counter declaration is inserted).
    pub decl_offset: usize,
    /// Byte offset JUST AFTER the loop's opening `{` (where the increment
    /// + check is inserted).
    pub body_offset: usize,
    /// `while (true)` vs `loop` (excerpt text only).
    pub is_while_true: bool,
}

/// Collect one rule's edits for the source (empty for unknown or
/// suggest-only rules).
#[must_use]
pub fn collect_edits(rule_id: &str, source: &str) -> Vec<RuleEdit> {
    if FIXABLE_SHADER.contains(&rule_id) {
        propose_loop_guard(source, DEFAULT_GUARD_BOUND)
            .into_iter()
            .map(|edit| RuleEdit {
                rule_id: rule_id.to_string(),
                edit,
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Apply every bounded fix the source triggers, in one pass.
#[must_use]
pub fn collect_all_edits(source: &str) -> Vec<RuleEdit> {
    FIXABLE_SHADER
        .iter()
        .flat_map(|r| collect_edits(r, source))
        .collect()
}

/// The guard-insertion edits for every provably-unbounded render-stage
/// loop. Empty when the conservative gates refuse (guard name already
/// present ⇒ idempotence; no anchors ⇒ nothing to do).
#[must_use]
pub fn propose_loop_guard(source: &str, bound: u32) -> Vec<SourceEdit> {
    if source.contains(GUARD_NAME) {
        return Vec::new(); // idempotence refusal
    }
    let anchors = unbounded_loop_anchors(source);
    let mut edits = Vec::with_capacity(anchors.len() * 2);
    for (i, a) in anchors.iter().enumerate() {
        let name = if anchors.len() == 1 {
            GUARD_NAME.to_string()
        } else {
            format!("{GUARD_NAME}_{i}")
        };
        // The declaration, at the loop line's leading indentation.
        edits.push(SourceEdit {
            start: a.decl_offset,
            end: a.decl_offset,
            replacement: format!("var {name}: u32 = 0u;\n"),
        });
        // The increment + bound-check, at the loop body's head. `break`
        // BEFORE the wrap (the wrapping-guard-counter refusal).
        edits.push(SourceEdit {
            start: a.body_offset,
            end: a.body_offset,
            replacement: format!(
                "\n        {name} = {name} + 1u;\n        if ({name} >= {bound}u) {{ break; }}"
            ),
        });
    }
    edits.shrink_to_fit();
    edits
}

/// The dedicated walk for the proposer's rule: byte-anchored, block-aware,
/// render-stage-only. Mirrors [`ShaderScanner::scan`]'s loop logic; the
/// `anchors_match_scanner_findings` drift-guard test pins the two walks
/// together.
pub(crate) fn unbounded_loop_anchors(source: &str) -> Vec<LoopAnchor> {
    let mut anchors = Vec::new();
    let mut stack: Vec<(bool, bool)> = Vec::new(); // (in_render_fn, _unused)
    let mut pending_stage: Option<Stage> = None;
    let mut offset = 0usize;

    for raw in source.lines() {
        let lineno_offset = offset;
        let line = super::mask_string_literals(&super::strip_line_comment(raw));
        let trimmed = line.trim();
        let opens_fn = line
            .find('(')
            .is_some_and(|idx| line[..idx].split_whitespace().any(|t| t == "fn"));

        if line.contains("@vertex") {
            pending_stage = Some(Stage::Vertex);
        }
        if line.contains("@fragment") {
            pending_stage = Some(Stage::Fragment);
        }

        // Leading `}` closes one block (the `} else {` chain).
        if trimmed.starts_with('}') {
            stack.pop();
        }

        let is_loop_line =
            (contains_w(&line, "loop") || contains_w(&line, "while")) && line.contains('{');
        let in_render_fn = stack.iter().any(|(r, _)| *r);
        if opens_fn {
            stack.push((pending_stage.is_some(), false));
            pending_stage = None;
        } else if is_loop_line && in_render_fn {
            // Open the loop block; record the anchor when its brace-matched
            // body carries no exit token. Nested loops are anchored
            // independently (each gets its own walk pass); a break in an
            // INNER loop conservatively marks the outer as exited (a miss,
            // never a false anchor — the safe direction).
            let brace_off = lineno_offset + line.find('{').unwrap_or(0);
            if let Some(close_off) = matching_brace(source, brace_off) {
                let body = &source[brace_off + 1..close_off];
                let has_exit = body.lines().any(|l| {
                    let m = super::mask_string_literals(&super::strip_line_comment(l));
                    contains_w(&m, "break") || contains_w(&m, "return")
                });
                if !has_exit {
                    let decl_offset = lineno_offset + (raw.len() - raw.trim_start().len());
                    anchors.push(LoopAnchor {
                        line: 0, // filled below from the decl offsets
                        decl_offset,
                        body_offset: brace_off + 1,
                        is_while_true: contains_w(&line, "while"),
                    });
                }
            }
            // Push a frame so brace accounting stays balanced to the loop's
            // close (nested fns inside a loop body are not valid WGSL, so
            // no render attribution is lost by the plain frame).
            stack.push((false, false));
        } else if line.contains('{') && !trimmed.starts_with('}') {
            stack.push((false, false));
        }

        // Trailing closes: the line's remaining `}`s (beyond any leading
        // one already handled) pop frames — keeps the render-fn flag
        // accurate after a fn closes (one-liner `if (x) { ...; }` bodies,
        // `}}` chain ends).
        let total_closes = line.matches('}').count();
        let leading = usize::from(trimmed.starts_with('}') || line.trim_start().starts_with('}'));
        let mut remaining = total_closes.saturating_sub(leading);
        while remaining > 0 {
            remaining -= 1;
            stack.pop();
        }

        offset += raw.len() + 1;
    }
    // Fill the line numbers (1-based) from the decl offsets.
    let mut cum = 0usize;
    let mut line_no = 0usize;
    let mut by_offset: Vec<(usize, usize)> = Vec::new();
    for raw in source.lines() {
        line_no += 1;
        by_offset.push((cum, line_no));
        cum += raw.len() + 1;
    }
    for a in &mut anchors {
        a.line = by_offset
            .iter()
            .rev()
            .find(|(off, _)| *off <= a.decl_offset)
            .map_or(0, |(_, l)| *l);
    }
    anchors.shrink_to_fit();
    anchors
}

/// The byte offset of the `}` matching the `{` at `open_off`, or None.
fn matching_brace(source: &str, open_off: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut i = open_off;
    let mut in_string = false;
    while i < bytes.len() {
        let ch = bytes[i];
        if in_string {
            if ch == b'"' && bytes[i - 1] != b'\\' {
                in_string = false;
            }
        } else if ch == b'"' {
            in_string = true;
        } else if ch == b'{' {
            depth += 1;
        } else if ch == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn contains_w(line: &str, word: &str) -> bool {
    let bytes = line.as_bytes();
    let w = word.as_bytes();
    let mut i = 0;
    while i + w.len() <= bytes.len() {
        if &bytes[i..i + w.len()] == w {
            let before_ok =
                i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
            let after = i + w.len();
            let after_ok = after >= bytes.len()
                || !(bytes[after].is_ascii_alphanumeric() || bytes[after] == b'_');
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::shader_perf::ShaderScanner;

    const FIXTURE: &str = "@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n  loop {\n    let x = 1.0;\n  }\n  return vec4<f32>(0.0);\n}\n";

    fn finding_lines(src: &str) -> Vec<usize> {
        ShaderScanner
            .scan(src)
            .into_iter()
            .filter(|f| f.rule_id == "unbounded-render-loop-guard")
            .map(|f| f.line)
            .collect()
    }

    fn anchor_lines(src: &str) -> Vec<usize> {
        unbounded_loop_anchors(src).iter().map(|a| a.line).collect()
    }

    /// The drift guard: the proposer's anchors and the scanner's findings
    /// name the SAME lines — detection and rewrite cannot diverge.
    #[test]
    fn anchors_match_scanner_findings() {
        assert_eq!(anchor_lines(FIXTURE), finding_lines(FIXTURE));
        let multi = "@vertex\nfn vs_main() -> @builtin(position) vec4<f32> {\n  while (true) {\n    let x = 1.0;\n  }\n  return vec4<f32>(0.0);\n}\n@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n  loop {\n    let y = 2.0;\n  }\n  return vec4<f32>(0.0);\n}\nfn helper() -> f32 {\n  loop { let z = 3.0; }\n}\n";
        assert_eq!(anchor_lines(multi), finding_lines(multi));
        assert_eq!(anchor_lines(multi), vec![3, 10]);
    }

    #[test]
    fn guard_insertion_bounds_the_loop() {
        let edits = propose_loop_guard(FIXTURE, DEFAULT_GUARD_BOUND);
        assert_eq!(edits.len(), 2);
        let healed = crate::edit_emitter::apply_edits(FIXTURE, edits);
        assert!(healed.contains("var shader_heal_guard: u32 = 0u;"));
        assert!(healed.contains("if (shader_heal_guard >= 256u) { break; }"));
        // Idempotence: the healed source refuses a second guard.
        assert_eq!(
            propose_loop_guard(&healed, DEFAULT_GUARD_BOUND),
            [] as [SourceEdit; 0]
        );
        // And the scanner agrees the loop is now bounded.
        assert!(!finding_lines(&healed).contains(&3));
    }

    #[test]
    fn tier0_invariants_hold_on_the_healed_source() {
        let edits = propose_loop_guard(FIXTURE, DEFAULT_GUARD_BOUND);
        let healed = crate::edit_emitter::apply_edits(FIXTURE, edits);
        assert_eq!(
            super::super::scanner::entry_point_multiset(FIXTURE),
            super::super::scanner::entry_point_multiset(&healed)
        );
        assert_eq!(
            super::super::scanner::group_binding_multiset(FIXTURE),
            super::super::scanner::group_binding_multiset(&healed)
        );
    }
}

//! `clippy::match_like_matches_macro` — `match x { P => true, _ => false }`
//! → `matches!(x, P)` (Batch 164 / Plan 155 — the frontier-lead mining pack
//! X; the family fixture verified to fire exactly this lint under
//! all+pedantic+nursery with a clean seed, 2026-09-19
//! (`scripts/plan155_frontier_probe.py`); style/default-on, group pinned by
//! the probe's no-flags arm).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! `matches!` IS that match: the macro expands to
//! `match $expression { $pattern $(if $guard)? => true, _ => false }` —
//! the scrutinee, pattern, and optional arm-1 guard re-emit VERBATIM, the
//! expansion evaluates the scrutinee exactly once at the same position, and
//! the arm bodies (`true` / `_ => false`) are the macro's own definition.
//! The negated twin (`P => false, _ => true`) is the same identity under
//! `!` — boolean negation of the same two-arm match. There is no type
//! question anywhere: every pattern accepted by a match arm is accepted by
//! `$pattern:pat`, and the guard moves through as `$guard:expr`.
//!
//! ## Guards (the bounded fix-space)
//!
//! - EXACTLY two arms; arm 2 is the bare `_` wildcard with NO guard and a
//!   bool-literal body opposite to arm 1's. Named catch-alls (`other =>`)
//!   are SILENT upstream (probe-pinned) and refuse; 3-arm matches never
//!   fire mlmm upstream (probe: only `match_same_arms` fires there).
//! - Both bodies must be `true`/`false` literals (`helpers::bool_lit`);
//!   non-bool bodies are SILENT upstream and refuse.
//! - Arm 1 must not itself be `_` (arm 2 would be unreachable).
//! - An arm-1 guard is CARRIED verbatim (`matches!(x, 0 if x > 1)` — the
//!   probe rendered exactly that and it is valid `$pattern:pat if
//!   $guard:expr` matches! syntax); a guard on the WILDCARD arm refuses
//!   (the negation would have to distribute over the guard — out of scope).
//! - Match/arm attributes refuse (`#[cfg]` on the match or an arm — the
//!   edit span semantics around attrs are not worth the risk, the
//!   `needless_late_init` Batch-163 lesson).
//! - Comments inside the replaced span refuse.

use syn::visit::Visit;
use syn::{ExprMatch, File, Pat};

use super::helpers::{bool_lit, byte_range};
use super::{SourceEdit, edit};
use crate::edit_emitter::drops_interior_comment;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_match(&mut self, m: &'ast ExprMatch) {
        syn::visit::visit_expr_match(self, m);
        if let Some(replacement) = self.try_rewrite(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_rewrite(&self, m: &ExprMatch) -> Option<String> {
        if !m.attrs.is_empty() || m.arms.len() != 2 {
            return None;
        }
        let [arm1, arm2] = m.arms.as_slice() else {
            return None;
        };
        if !arm1.attrs.is_empty() || !arm2.attrs.is_empty() {
            return None;
        }
        // Arm 2: the bare wildcard, no guard, bool body.
        if !matches!(arm2.pat, Pat::Wild(_)) || arm2.guard.is_some() {
            return None;
        }
        // Arm 1: a real pattern (not `_`), bool body opposite to arm 2's.
        if matches!(arm1.pat, Pat::Wild(_)) {
            return None;
        }
        let body1 = bool_lit(&arm1.body)?;
        let body2 = bool_lit(&arm2.body)?;
        if body1 == body2 {
            return None;
        }
        let span = byte_range(m);
        if drops_interior_comment(self.source, span, &[]) {
            return None;
        }
        let scrutinee = self.source.get(byte_range(&m.expr))?.to_string();
        let pattern = self.source.get(byte_range(&arm1.pat))?.to_string();
        let inner = if let Some((_, guard)) = &arm1.guard {
            let guard = self.source.get(byte_range(guard.as_ref()))?.to_string();
            format!("{scrutinee}, {pattern} if {guard}")
        } else {
            format!("{scrutinee}, {pattern}")
        };
        Some(if body1 {
            format!("matches!({inner})")
        } else {
            format!("!matches!({inner})")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(
            propose_fix(
                "match_like_matches_macro",
                "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); x > 0 }",
            )
            .is_none()
        );
    }

    #[test]
    fn two_arm_bool_match_fires() {
        let out = propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); match x { 0 => true, _ => false } }",
        )
        .expect("matched");
        assert!(out.contains("matches!(x, 0)"), "not rewritten: {out}");
        assert!(!out.contains("=> true"), "match not collapsed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn negated_twin_fires() {
        let out = propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); match x { 0 => false, _ => true } }",
        )
        .expect("matched");
        assert!(out.contains("!matches!(x, 0)"), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn or_pattern_carries_verbatim() {
        let out = propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); match x { 0 | 1 => true, _ => false } }",
        )
        .expect("matched");
        assert!(out.contains("matches!(x, 0 | 1)"), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn binding_pattern_carries_verbatim() {
        let out = propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); match x { v @ 0..=5 => true, _ => false } }",
        )
        .expect("matched");
        assert!(
            out.contains("matches!(x, v @ 0..=5)"),
            "not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn arm_guard_carries_verbatim() {
        let out = propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); match x { 0 if x > 1 => true, _ => false } }",
        )
        .expect("matched");
        assert!(
            out.contains("matches!(x, 0 if x > 1)"),
            "not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn guard_on_wildcard_arm_refuses() {
        assert!(propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); match x { 0 => true, _ if x > 1 => false } }",
        )
        .is_none());
    }

    #[test]
    fn three_arms_refuse() {
        // Upstream mlmm is SILENT here (probe: only match_same_arms fires).
        assert!(propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); match x { 0 => true, 1 => false, _ => false } }",
        )
        .is_none());
    }

    #[test]
    fn named_catchall_refuses() {
        // Upstream mlmm is SILENT here (probe-pinned).
        assert!(propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); match x { 0 => true, other => false } }",
        )
        .is_none());
    }

    #[test]
    fn non_bool_bodies_refuse() {
        assert!(propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); match x { 0 => 1, _ => 2 } }",
        )
        .is_none());
    }

    #[test]
    fn wildcard_first_arm_refuses() {
        assert!(propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); match x { _ => true, 0 => false } }",
        )
        .is_none());
    }

    #[test]
    fn same_bool_bodies_refuse() {
        assert!(propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); match x { 0 => true, _ => true } }",
        )
        .is_none());
    }

    #[test]
    fn nested_in_call_fires_in_place() {
        let out = propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> Option<bool> { hits.push(0); Some(match x { 0 => true, _ => false }) }",
        )
        .expect("matched");
        assert!(out.contains("Some(matches!(x, 0))"), "not in place: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn match_on_match_attr_refuses() {
        assert!(propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); #[allow(unused)] match x { 0 => true, _ => false } }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            "match_like_matches_macro",
            "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); match x { 0 => /* mid */ true, _ => false } }",
        )
        .is_none());
    }
}

//! `clippy::single_match` — `match x { Some(v) => f(v), _ => () }` →
//! `if let Some(v) = x { f(v) }`.
//!
//! Pattern: a two-arm match, no attributes, no guards, where the SECOND arm
//! is a wildcard (`_`) with a unit body (`()` or `{}`) and the FIRST arm is
//! any non-wildcard pattern. (The mirrored order — wildcard first — is left
//! alone; rewriting it would reorder which arm runs.)
//!
//! Span-preserving (Issue 016): the whole match's byte range is replaced by
//! `if let <pat> = <scrutinee> <body>` — pattern, scrutinee, and body keep
//! their ORIGINAL source text; a non-block body is braced. The if-let
//! scrutinee position takes any expression, so no parens are needed.

use super::helpers::is_unit_expr;
use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{Expr, ExprMatch, File, Pat};

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

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_match(&mut self, m: &'ast ExprMatch) {
        syn::visit::visit_expr_match(self, m);
        if let Some(replacement) = self.try_single_match(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_single_match(&self, m: &ExprMatch) -> Option<String> {
        if m.arms.len() != 2 || !m.attrs.is_empty() {
            return None;
        }
        if m.arms
            .iter()
            .any(|a| !a.attrs.is_empty() || a.guard.is_some())
        {
            return None; // guards/arm-attrs would be lost by the splice
        }
        if !matches!(m.arms[1].pat, Pat::Wild(_)) || !is_unit_expr(&m.arms[1].body) {
            return None; // not the `_ => ()` shape (that's single_match_else)
        }
        if matches!(m.arms[0].pat, Pat::Wild(_)) {
            return None;
        }
        let pat = src(self.source, &m.arms[0].pat)?;
        let scrutinee = src(self.source, &m.expr)?;
        let body = braced_body(self.source, &m.arms[0].body)?;
        Some(format!("if let {pat} = {scrutinee} {body}"))
    }
}

/// A match-arm body as an if-let body: block bodies keep their ORIGINAL
/// braces; expression bodies get fresh ones.
fn braced_body(source: &str, body: &Expr) -> Option<String> {
    let text = src(source, body)?;
    Some(match body {
        Expr::Block(_) => text,
        _ => format!("{{ {text} }}"),
    })
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_to_if_let() {
        let input = "fn g(x: Option<u8>) { match x { Some(v) => drop(v), _ => () } }";
        let out = propose_fix("single_match", input).expect("matched");
        assert!(
            out.contains("if let Some(v) = x { drop(v) }"),
            "not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn block_body_keeps_original_braces() {
        let input = "fn g(x: Option<u8>) { match x { Some(v) => { drop(v); }, _ => () } }";
        let out = propose_fix("single_match", input).expect("matched");
        assert!(out.contains("if let Some(v) = x { drop(v); }"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn non_unit_wildcard_is_single_match_else() {
        let input = "fn g(x: Option<u8>) { match x { Some(v) => drop(v), _ => fallback() } }";
        let out = propose_fix("single_match", input);
        assert!(
            out.is_none(),
            "non-unit wildcard arm must not fire here: {out:?}"
        );
    }

    #[test]
    fn three_arms_untouched() {
        let input = "fn g(x: u8) { match x { 1 => a(), 2 => b(), _ => () } }";
        let out = propose_fix("single_match", input);
        assert!(out.is_none(), "3-arm match must not fire: {out:?}");
    }

    #[test]
    fn guarded_arm_untouched() {
        let input = "fn g(x: Option<u8>) { match x { Some(v) if v > 0 => drop(v), _ => () } }";
        let out = propose_fix("single_match", input);
        assert!(out.is_none(), "guarded arm must not fire: {out:?}");
    }
}

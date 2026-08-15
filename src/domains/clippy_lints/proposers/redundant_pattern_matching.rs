//! `clippy::redundant_pattern_matching` — `if let Ok(_) = r { .. }` →
//! `if r.is_ok() { .. }` (also `while let`).
//!
//! Pattern: an `if let` / `while let` whose pattern is a wildcard-only
//! constructor test — `Ok(_)`, `Err(_)`, `Some(_)` (tuple-struct with one
//! wildcard-or-rest element) or bare `None`. Patterns with BINDINGS
//! (`Some(v)`, `Ok(v)`) keep the binding and do NOT fire.
//!
//! Span-preserving (Issue 016): the let-condition node's byte range
//! (`let Pat = Scrut`) is replaced by `<scrut>.<is_pred>()`; the scrutinee
//! keeps its ORIGINAL source text, parenthesized when its form doesn't bind
//! tighter than `.`.

use super::helpers::{is_none_pat, is_postfix_safe};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprIf, ExprLet, ExprWhile, File, Pat};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector { source, edits: Vec::new() };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_if(&mut self, i: &'ast ExprIf) {
        syn::visit::visit_expr_if(self, i);
        if let Expr::Let(l) = &*i.cond {
            if let Some(e) = self.try_rewrite(l) {
                self.edits.push(e);
            }
        }
    }

    fn visit_expr_while(&mut self, w: &'ast ExprWhile) {
        syn::visit::visit_expr_while(self, w);
        if let Expr::Let(l) = &*w.cond {
            if let Some(e) = self.try_rewrite(l) {
                self.edits.push(e);
            }
        }
    }
}

impl Collector<'_> {
    fn try_rewrite(&self, l: &ExprLet) -> Option<SourceEdit> {
        let pred = constructor_pred(&l.pat)?;
        let scrutinee = src(self.source, &*l.expr)?;
        let scrutinee = if is_postfix_safe(&l.expr) { scrutinee } else { format!("({scrutinee})") };
        Some(edit(l, format!("{scrutinee}.{pred}()")))
    }
}

/// The `is_*` predicate this let-pattern reduces to, if it is the
/// wildcard-only form. `None` for binding patterns — the binding is used,
/// not redundant.
fn constructor_pred(pat: &Pat) -> Option<&'static str> {
    if let Pat::TupleStruct(ts) = pat {
        if ts.elems.len() == 1 && matches!(ts.elems[0], Pat::Wild(_) | Pat::Rest(_)) {
            if ts.path.is_ident("Some") {
                return Some("is_some");
            }
            if ts.path.is_ident("Ok") {
                return Some("is_ok");
            }
            if ts.path.is_ident("Err") {
                return Some("is_err");
            }
        }
        return None;
    }
    is_none_pat(pat).then_some("is_none")
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn if_let_ok_wildcard_becomes_is_ok() {
        let input = "fn f(r: Result<u8, u8>) { if let Ok(_) = r {} }";
        let out = propose_fix("redundant_pattern_matching", input).expect("matched");
        assert!(out.contains("if r.is_ok() {}"), "not converted: {out}");
        assert!(!out.contains("let"), "let must be gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn while_let_some_wildcard_becomes_is_some() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) { while let Some(_) = it.next() {} }";
        let out = propose_fix("redundant_pattern_matching", input).expect("matched");
        assert!(out.contains("while it.next().is_some() {}"), "not converted: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn if_let_err_wildcard_becomes_is_err() {
        let input = "fn f(r: Result<u8, u8>) { if let Err(_) = r {} }";
        let out = propose_fix("redundant_pattern_matching", input).expect("matched");
        assert!(out.contains("if r.is_err() {}"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn if_let_none_becomes_is_none() {
        let input = "fn f(x: Option<u8>) { if let None = x {} }";
        let out = propose_fix("redundant_pattern_matching", input).expect("matched");
        assert!(out.contains("if x.is_none() {}"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn option_binding_pattern_untouched() {
        let input = "fn f(x: Option<u8>) { if let Some(v) = x { drop(v); } }";
        let out = propose_fix("redundant_pattern_matching", input);
        assert!(out.is_none(), "binding pattern must not fire: {out:?}");
    }

    #[test]
    fn result_binding_pattern_untouched() {
        let input = "fn f(y: Result<u8, u8>) { if let Ok(v) = y { drop(v); } }";
        let out = propose_fix("redundant_pattern_matching", input);
        assert!(out.is_none(), "binding pattern must not fire: {out:?}");
    }
}

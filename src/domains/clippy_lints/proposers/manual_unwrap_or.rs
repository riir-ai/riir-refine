//! `clippy::manual_unwrap_or` — `match x { Some(v) => v, None => 7 }` →
//! `x.unwrap_or(7)`.
//!
//! Pattern: a two-arm match, no attributes, no guards, either order:
//! - arm A: `Some(v) => v` (identity binding use ONLY)
//! - arm B: `None => <default>` where the default is PURE (paths, literals,
//!   field access, paren/binary/unary compositions) — a call default would
//!   change evaluation timing when moved into eager argument position, so it
//!   is conservatively rejected.
//!
//! Span-preserving (Issue 016): the whole match's byte range is replaced by
//! `<scrutinee>.unwrap_or(<default>)` — both fragments keep their ORIGINAL
//! source text; the scrutinee is parenthesized when its form doesn't bind
//! tighter than `.`.

use super::helpers::{is_none_pat, is_postfix_safe, is_pure_expr};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Arm, Expr, ExprMatch, File, Pat};

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
        if let Some(replacement) = self.try_unwrap_or(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_unwrap_or(&self, m: &ExprMatch) -> Option<String> {
        if m.arms.len() != 2 || !m.attrs.is_empty() {
            return None;
        }
        if m.arms
            .iter()
            .any(|a| !a.attrs.is_empty() || a.guard.is_some())
        {
            return None; // guards/arm-attrs would be lost by the splice
        }
        let scrutinee = src(self.source, &*m.expr)?;
        let scrutinee = if is_postfix_safe(&m.expr) {
            scrutinee
        } else {
            format!("({scrutinee})")
        };
        for (a, b) in [(&m.arms[0], &m.arms[1]), (&m.arms[1], &m.arms[0])] {
            if is_some_identity(a) && is_none_pat(&b.pat) && is_pure_expr(&b.body) {
                let default = src(self.source, &b.body)?;
                return Some(format!("{scrutinee}.unwrap_or({default})"));
            }
        }
        None
    }
}

/// `Some(v) => v` — tuple-struct Some with one binding whose body is that
/// same binding.
fn is_some_identity(arm: &Arm) -> bool {
    let Some(binding) = single_ctor_binding(&arm.pat, "Some") else {
        return false;
    };
    matches!(&*arm.body, Expr::Path(p) if p.path.get_ident() == Some(&binding))
}

/// The single `Ctor(<binding>)` pattern's binding ident.
fn single_ctor_binding(pat: &Pat, ctor: &str) -> Option<syn::Ident> {
    let Pat::TupleStruct(ts) = pat else {
        return None;
    };
    if !ts.path.is_ident(ctor) || ts.elems.len() != 1 {
        return None;
    }
    match &ts.elems[0] {
        Pat::Ident(pi) => Some(pi.ident.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn literal_default_becomes_unwrap_or() {
        let input = "fn f(x: Option<u8>) -> u8 { match x { Some(v) => v, None => 7 } }";
        let out = propose_fix("manual_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.unwrap_or(7)"),
            "unwrap_or not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn none_first_arm_order_accepted() {
        let input = "fn f(x: Option<u8>) -> u8 { match x { None => 7, Some(v) => v } }";
        let out = propose_fix("manual_unwrap_or", input).expect("matched");
        assert!(out.contains("x.unwrap_or(7)"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn deref_scrutinee_gets_parens() {
        let input = "fn f(r: &Option<u8>) -> u8 { match *r { Some(v) => v, None => 7 } }";
        let out = propose_fix("manual_unwrap_or", input).expect("matched");
        assert!(
            out.contains("(*r).unwrap_or(7)"),
            "deref must be parenthesized: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn pure_path_default_fires() {
        let input = "fn f(x: Option<u8>, d: u8) -> u8 { match x { Some(v) => v, None => d } }";
        let out = propose_fix("manual_unwrap_or", input).expect("matched");
        assert!(out.contains("x.unwrap_or(d)"), "{out}");
    }

    #[test]
    fn impure_call_default_untouched() {
        // A call default would be evaluated eagerly in argument position —
        // timing the original match only pays on the None path.
        let input = "fn f(x: Option<u8>) -> u8 { match x { Some(v) => v, None => compute() } }";
        let out = propose_fix("manual_unwrap_or", input);
        assert!(out.is_none(), "impure default must not fire: {out:?}");
    }

    #[test]
    fn non_identity_some_body_untouched() {
        let input = "fn f(x: Option<u8>) -> u8 { match x { Some(v) => v + 1, None => 7 } }";
        let out = propose_fix("manual_unwrap_or", input);
        assert!(
            out.is_none(),
            "non-identity Some body must not fire: {out:?}"
        );
    }

    #[test]
    fn three_arms_untouched() {
        let input = "fn f(x: Option<u8>) -> u8 { match x { Some(v) => v, None => 7, _ => 0 } }";
        let out = propose_fix("manual_unwrap_or", input);
        assert!(out.is_none(), "3-arm match must not fire: {out:?}");
    }
}

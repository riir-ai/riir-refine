//! `clippy::needless_match` — `match x { Some(v) => Some(v), None => None }`
//! → `x` (the identity match; same for `Ok`/`Err` and `true`/`false`).
//!
//! Pattern: a two-arm match, no attributes, no guards, that merely rebuilds
//! its scrutinee (either arm order):
//! - Option: `Some(v) => Some(v)` + `None => None`
//! - Result: `Ok(v) => Ok(v)` + `Err(e) => Err(e)`
//! - bool: `true => true` + `false => false`
//!
//! Span-preserving (Issue 016): the whole match's byte range is replaced by
//! the scrutinee's ORIGINAL source text, parenthesized when its form doesn't
//! bind tighter than postfix/`?`-style continuation (a deref scrutinee must
//! not become `*r` in expression position where `r` would read as the value).

use super::helpers::{bool_lit, is_none_pat, is_postfix_safe};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Arm, Expr, ExprMatch, File, Lit, Pat};

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
        if let Some(replacement) = self.try_identity(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_identity(&self, m: &ExprMatch) -> Option<String> {
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
            if (is_ctor_rebuild(a, "Some") && is_none_literal_arm(b))
                || (is_ctor_rebuild(a, "Ok") && is_ctor_rebuild(b, "Err"))
                || (is_ctor_rebuild(a, "Err") && is_ctor_rebuild(b, "Ok"))
                || (is_bool_arm(a, true) && is_bool_arm(b, false))
            {
                return Some(scrutinee);
            }
        }
        None
    }
}

/// `Ctor(v) => Ctor(v)` — tuple-struct pattern with one binding, rebuilt by
/// a call to the same constructor with that same binding.
fn is_ctor_rebuild(arm: &Arm, ctor: &str) -> bool {
    let Some(binding) = single_ctor_binding(&arm.pat, ctor) else {
        return false;
    };
    match &*arm.body {
        Expr::Call(c) => {
            matches!(&*c.func, Expr::Path(p) if p.path.is_ident(ctor))
                && c.args.len() == 1
                && matches!(&c.args[0], Expr::Path(p) if p.path.get_ident() == Some(&binding))
        }
        _ => false,
    }
}

/// `None => None`
fn is_none_literal_arm(arm: &Arm) -> bool {
    is_none_pat(&arm.pat) && matches!(&*arm.body, Expr::Path(p) if p.path.is_ident("None"))
}

/// `<bool-literal> => <same bool-literal>`
fn is_bool_arm(arm: &Arm, want: bool) -> bool {
    pat_is_bool(&arm.pat, want) && bool_lit(&arm.body) == Some(want)
}

/// Is this pattern the boolean literal `true`/`false`?
fn pat_is_bool(p: &Pat, want: bool) -> bool {
    matches!(p, Pat::Lit(pl) if matches!(&pl.lit, Lit::Bool(b) if b.value == want))
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
    fn option_identity_collapses() {
        let input =
            "fn f(x: Option<u8>) -> Option<u8> { match x { Some(v) => Some(v), None => None } }";
        let out = propose_fix("needless_match", input).expect("matched");
        assert!(out.contains("{ x }"), "identity not collapsed: {out}");
        assert!(!out.contains("match"), "match must be gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn result_identity_collapses() {
        let input =
            "fn g(r: Result<u8, u16>) -> Result<u8, u16> { match r { Ok(v) => Ok(v), Err(e) => Err(e) } }";
        let out = propose_fix("needless_match", input).expect("matched");
        assert!(out.contains("{ r }"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn bool_identity_collapses() {
        let input = "fn h(b: bool) -> bool { match b { true => true, false => false } }";
        let out = propose_fix("needless_match", input).expect("matched");
        assert!(out.contains("{ b }"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn none_first_arm_order_accepted() {
        let input =
            "fn f(x: Option<u8>) -> Option<u8> { match x { None => None, Some(v) => Some(v) } }";
        let out = propose_fix("needless_match", input).expect("matched");
        assert!(out.contains("{ x }"), "{out}");
    }

    #[test]
    fn deref_scrutinee_gets_parens() {
        let input =
            "fn f(r: &Option<u8>) -> Option<u8> { match *r { Some(v) => Some(v), None => None } }";
        let out = propose_fix("needless_match", input).expect("matched");
        assert!(
            out.contains("{ (*r) }"),
            "deref must be parenthesized: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn transformed_ctor_body_untouched() {
        let input =
            "fn f(x: Option<u8>) -> Option<u8> { match x { Some(v) => Some(v + 1), None => None } }";
        let out = propose_fix("needless_match", input);
        assert!(
            out.is_none(),
            "non-identity ctor body must not fire: {out:?}"
        );
    }

    #[test]
    fn unboxed_body_untouched() {
        let input = "fn f(x: Option<u8>) -> Option<u8> { match x { Some(v) => v, None => None } }";
        let out = propose_fix("needless_match", input);
        assert!(
            out.is_none(),
            "unboxed body is a different lint's shape: {out:?}"
        );
    }

    #[test]
    fn crossed_bool_arms_untouched() {
        // `true => false, false => true` is a negation, not an identity.
        let input = "fn h(b: bool) -> bool { match b { true => false, false => true } }";
        let out = propose_fix("needless_match", input);
        assert!(out.is_none(), "negation arms must not fire: {out:?}");
    }
}

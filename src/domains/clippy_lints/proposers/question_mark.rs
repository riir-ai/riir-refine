//! `clippy::question_mark` — `if x.is_none() { return None; }` → `x?;`, and
//! `match x { Some(v) => v, None => return None }` → `x?`.
//!
//! Two shapes:
//! 1. **if-stmt**: an if with no else whose condition is `<recv>.is_none()`
//!    (zero args) and whose then-block is EXACTLY `return None;`. The whole
//!    statement is replaced by `<recv>?;`.
//! 2. **match**: a two-arm match (either order) with `Some(v) => v` (body is
//!    that SAME binding) and `None => return None`. The whole match is
//!    replaced by `<scrut>?`.
//!
//! Span-preserving (Issue 016): receivers/scrutinees keep their ORIGINAL
//! source text, parenthesized when their form doesn't bind tighter than `?`.
//! A match can never sit in direct else-position (`else match` is invalid
//! Rust), so the bare `x?` replacement is structurally safe; the if-shape
//! lands in statement position, which takes any expression.
//!
//! Reference scrutinees (the `match &self.field { Some(s) => s, None =>
//! return None }` idiom — match ergonomics bind `s: &T`) are rewritten to
//! `inner.as_ref()` + try (the `riir-train` `gzero_loop` lesson): the try operator
//! needs `Option<T>` by value, so `(&self.field)?` is an E0277. Real
//! clippy's own suggestion for the shape (borrowing the field, then try)
//! is `MaybeIncorrect` for the same reason — it moves out of the shared
//! ref. `as_ref()` reproduces the `&T` binding
//! exactly and compiles for every payload. A plain-path scrutinee that is
//! TYPE-inferred to `&Option<T>` (invisible at parse time) keeps clippy's
//! diagnosis; the rare mis-fire surfaces as E0277 in the compile gate.

use super::helpers::{is_none_pat, is_postfix_safe};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Arm, Expr, ExprIf, ExprMatch, ExprReturn, File, Pat, Stmt};

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
    fn visit_stmt(&mut self, s: &'ast Stmt) {
        syn::visit::visit_stmt(self, s);
        let Stmt::Expr(Expr::If(i), _) = s else {
            return;
        };
        if let Some(e) = self.try_if_shape(s, i) {
            self.edits.push(e);
        }
    }

    fn visit_expr_match(&mut self, m: &'ast ExprMatch) {
        syn::visit::visit_expr_match(self, m);
        if let Some(e) = self.try_match_shape(m) {
            self.edits.push(e);
        }
    }
}

impl Collector<'_> {
    /// `if <recv>.is_none() { return None; }` → `<recv>?;`
    fn try_if_shape(&self, stmt: &Stmt, i: &ExprIf) -> Option<SourceEdit> {
        if i.else_branch.is_some() || !i.attrs.is_empty() {
            return None;
        }
        let Expr::MethodCall(mc) = &*i.cond else {
            return None;
        };
        if mc.method != "is_none" || !mc.args.is_empty() {
            return None;
        }
        let [Stmt::Expr(Expr::Return(r), Some(_))] = i.then_branch.stmts.as_slice() else {
            return None; // then-block must be EXACTLY `return None;`
        };
        if !returns_none(r) {
            return None;
        }
        let recv = self.try_operand(&mc.receiver)?;
        Some(edit(stmt, format!("{recv}?;")))
    }

    /// `match x { Some(v) => v, None => return None }` (either order) → `x?`
    fn try_match_shape(&self, m: &ExprMatch) -> Option<SourceEdit> {
        if m.arms.len() != 2 || !m.attrs.is_empty() {
            return None;
        }
        if m.arms
            .iter()
            .any(|a| !a.attrs.is_empty() || a.guard.is_some())
        {
            return None;
        }
        for (a, b) in [(&m.arms[0], &m.arms[1]), (&m.arms[1], &m.arms[0])] {
            if is_some_identity(a) && is_none_return(b) {
                let scrutinee = self.try_operand(&m.expr)?;
                return Some(edit(m, format!("{scrutinee}?")));
            }
        }
        None
    }

    /// The `?`-operand text for a receiver/scrutinee: original source,
    /// parenthesized when it doesn't bind tighter than `?` — except a
    /// REFERENCE operand, which becomes `<inner>.as_ref()` (parens on the
    /// inner when IT isn't postfix-safe) so `?` gets `Option<T>` by value
    /// while the bound value stays `&T` (match-ergonomics parity).
    fn try_operand(&self, expr: &Expr) -> Option<String> {
        let mut inner = expr;
        while let Expr::Paren(p) = inner {
            inner = &p.expr;
        }
        if let Expr::Reference(r) = inner {
            let text = src(self.source, &*r.expr)?;
            let text = if is_postfix_safe(&r.expr) {
                text
            } else {
                format!("({text})")
            };
            return Some(format!("{text}.as_ref()"));
        }
        let text = src(self.source, expr)?;
        let text = if is_postfix_safe(expr) {
            text
        } else {
            format!("({text})")
        };
        Some(text)
    }
}

/// Is this a `return None` (bare `None` path, no other expr)?
fn returns_none(r: &ExprReturn) -> bool {
    matches!(&r.expr, Some(e) if matches!(&**e, Expr::Path(p) if p.path.is_ident("None")))
}

/// `Some(v) => v` — tuple-struct Some with one binding whose body is that
/// same binding.
fn is_some_identity(arm: &Arm) -> bool {
    let Some(binding) = single_ctor_binding(&arm.pat, "Some") else {
        return false;
    };
    matches!(&*arm.body, Expr::Path(p) if p.path.get_ident() == Some(&binding))
}

/// `None => return None`
fn is_none_return(arm: &Arm) -> bool {
    is_none_pat(&arm.pat) && matches!(&*arm.body, Expr::Return(r) if returns_none(r))
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
    fn if_is_none_return_none_becomes_try() {
        let input =
            "fn f(x: Option<u8>) -> Option<u8> { if x.is_none() { return None; } Some(x.unwrap() + 1) }";
        let out = propose_fix("question_mark", input).expect("matched");
        assert!(out.contains("x?;"), "try operator not produced: {out}");
        assert!(
            out.contains("Some(x.unwrap() + 1)"),
            "tail must survive: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn match_some_v_none_return_becomes_try() {
        let input =
            "fn g(x: Option<u8>) -> Option<u8> { match x { Some(v) => v, None => return None, } }";
        let out = propose_fix("question_mark", input).expect("matched");
        assert!(out.contains("x?"), "try operator not produced: {out}");
        assert!(!out.contains("match"), "match must be gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn none_first_arm_order_accepted() {
        let input =
            "fn g(x: Option<u8>) -> Option<u8> { match x { None => return None, Some(v) => v, } }";
        let out = propose_fix("question_mark", input).expect("matched");
        assert!(out.contains("x?"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn deref_scrutinee_gets_parens() {
        let input =
            "fn g(r: &Option<u8>) -> Option<u8> { match *r { Some(v) => v, None => return None, } }";
        let out = propose_fix("question_mark", input).expect("matched");
        assert!(out.contains("(*r)?"), "deref must be parenthesized: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn ref_scrutinee_field_becomes_as_ref_try() {
        // The production shape (riir-train gzero_loop): match ergonomics bind
        // `s: &T`, so the rewrite must keep the borrow — `(&self.opt)?` is an
        // E0277 (`?` needs `Option<T>` by value) and real clippy's own
        // `&self.opt?` suggestion moves out of the shared ref.
        let input = "impl S { fn f(&self) -> Option<u8> { let v = match &self.opt { Some(s) => s, None => return None, }; Some(*v) } }";
        let out = propose_fix("question_mark", input).expect("matched");
        assert!(
            out.contains("self.opt.as_ref()?"),
            "reference scrutinee must become .as_ref()?: {out}"
        );
        assert!(!out.contains("match"), "match must be gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn ref_scrutinee_deref_inner_gets_parens() {
        let input =
            "fn g(r: &Option<u8>) -> Option<u8> { match &*r { Some(v) => v, None => return None, } }";
        let out = propose_fix("question_mark", input).expect("matched");
        assert!(
            out.contains("(*r).as_ref()?"),
            "non-postfix-safe inner must be parenthesized before .as_ref(): {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn if_with_else_untouched() {
        let input =
            "fn f(x: Option<u8>) -> Option<u8> { if x.is_none() { return None; } else { return Some(1); } }";
        let out = propose_fix("question_mark", input);
        assert!(out.is_none(), "else branch must block the rewrite: {out:?}");
    }

    #[test]
    fn transformed_some_body_untouched() {
        let input =
            "fn g(x: Option<u8>) -> Option<u8> { match x { Some(v) => v + 1, None => return None, } }";
        let out = propose_fix("question_mark", input);
        assert!(
            out.is_none(),
            "non-identity Some body must not fire: {out:?}"
        );
    }
}

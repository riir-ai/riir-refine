//! `clippy::question_mark` — `if x.is_none() { return None; }` → `x?;`,
//! `match x { Some(v) => v, None => return None }` → `x?`, and
//! `let Some(v) = x else { return None }` → `let v = x?;`.
//!
//! Three shapes:
//! 1. **if-stmt**: an if with no else whose condition is `<recv>.is_none()`
//!    (zero args) and whose then-block is EXACTLY `return None;`. The whole
//!    statement is replaced by `<recv>?;`.
//! 2. **match**: a two-arm match (either order) with `Some(v) => v` (body is
//!    that SAME binding) and `None => return None`. The whole match is
//!    replaced by `<scrut>?`.
//! 3. **let-else** (P51, 2026-10-14 — clippy 1.98.1 fires "this `let...else`
//!    may be rewritten with the `?` operator", probe-verified):
//!    `let Some(v) = <init> else { return None };` → `let v = <init>?;`.
//!    The binding must be a plain ident (`mut` moves to the let: `Some(mut
//!    v)` → `let mut v`); ref/subpat/nested patterns refuse. A `&mut` init
//!    refuses (the shared `try_operand` re-emits references via `.as_ref()`,
//!    which would reborrow `&mut` down to `&` — the documented v1
//!    divergence; the `&` init keeps match-ergonomics parity via
//!    `.as_ref()`). Block-like inits (match/if) are grammar-forced into
//!    parens in let-else (syn + rustc both refuse the bare form — probed
//!    2026-10-14), so the operand law's paren strip/re-add covers them.
//!    Comment guard: only the binding and the init survive verbatim — a
//!    comment anywhere else in the Local refuses.
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

use super::helpers::{byte_range, drops_interior_comment, is_none_pat, is_postfix_safe};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Arm, Expr, ExprIf, ExprMatch, ExprReturn, File, Local, Pat, Stmt};

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
        let replacement = match s {
            Stmt::Expr(Expr::If(i), _) => self.try_if_shape(s, i),
            Stmt::Local(l) => self.try_let_else_shape(s, l),
            _ => None,
        };
        if let Some(e) = replacement {
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

    /// `let Some(v) = <init> else { return None };` → `let v = <init>?;`
    ///
    /// The else block must be EXACTLY one `return None` statement (the
    /// diverging-void / panic! / `return Err(..)` forms stay silent — they
    /// are not the `?` shape). Same operand law as the other shapes; the
    /// whole Local statement (span includes the trailing `;`) is one atomic
    /// edit. `&mut` inits refuse (see the module doc).
    fn try_let_else_shape(&self, stmt: &Stmt, l: &Local) -> Option<SourceEdit> {
        if !l.attrs.is_empty() {
            return None;
        }
        let init = l.init.as_ref()?;
        let Some((_else_tok, diverge)) = &init.diverge else {
            return None;
        };
        let Expr::Block(b) = &**diverge else {
            return None;
        };
        let [Stmt::Expr(Expr::Return(r), _)] = b.block.stmts.as_slice() else {
            return None; // else-block must be EXACTLY `return None` (semi optional)
        };
        if !returns_none(r) {
            return None;
        }
        let (binding, binding_pat, is_mut) = some_plain_binding(&l.pat)?;
        // `&mut` init: the shared operand law re-emits via `.as_ref()`,
        // reborrowing `&mut` down to `&` — refuse (v1 divergence).
        if let Expr::Reference(ri) = &*init.expr {
            if ri.mutability.is_some() {
                return None;
            }
        }
        // Comment guard: only the binding and the init survive verbatim.
        let kept = [byte_range(binding_pat), byte_range(&*init.expr)];
        if drops_interior_comment(self.source, byte_range(l), &kept) {
            return None;
        }
        let operand = self.try_operand(&init.expr)?;
        let mut_kw = if is_mut { "mut " } else { "" };
        Some(edit(stmt, format!("let {mut_kw}{binding} = {operand}?;")))
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

/// `Some(v)` / `Some(mut v)` — the binding ident, the binding PATTERN node
/// (for the comment guard's kept range), and whether it is `mut`.
/// Ref (`Some(ref v)`), subpat (`Some(v @ ..)`), and nested patterns refuse
/// (the let-else v1 shape).
fn some_plain_binding(pat: &Pat) -> Option<(syn::Ident, &Pat, bool)> {
    let Pat::TupleStruct(ts) = pat else {
        return None;
    };
    if !ts.path.is_ident("Some") || ts.elems.len() != 1 {
        return None;
    }
    let Pat::Ident(pi) = &ts.elems[0] else {
        return None;
    };
    if pi.by_ref.is_some() || pi.subpat.is_some() {
        return None;
    }
    Some((pi.ident.clone(), &ts.elems[0], pi.mutability.is_some()))
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
    fn let_else_return_none_becomes_try() {
        // The P51 production specimen shape (clone_on_copy.rs, cycle-71
        // probe): `let Some(arg) = c.args.first() else { return None };` →
        // `let arg = c.args.first()?;` — clippy 1.98.1 fires "this `let...else`
        // may be rewritten with the `?` operator" on it (probe-verified).
        let input =
            "fn f(c: Vec<u8>) -> Option<u8> { let Some(arg) = c.first() else { return None }; Some(arg + 1) }";
        let out = propose_fix("question_mark", input).expect("matched");
        assert!(
            out.contains("let arg = c.first()?;"),
            "let-else must become a try let: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn let_else_mut_binding_moves_mut_to_let() {
        let input =
            "fn f(x: Option<Vec<u8>>) -> Option<()> { let Some(mut v) = x else { return None }; v.push(1); Some(()) }";
        let out = propose_fix("question_mark", input).expect("matched");
        assert!(
            out.contains("let mut v = x?;"),
            "pattern mut must move onto the let: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn let_else_ref_init_becomes_as_ref_try() {
        // Match-ergonomics parity: `let Some(s) = &self.opt` binds s: &T;
        // `self.opt.as_ref()?` re-produces the &T binding.
        let input =
            "impl S { fn f(&self) -> Option<u8> { let Some(s) = &self.opt else { return None }; Some(*s) } }";
        let out = propose_fix("question_mark", input).expect("matched");
        assert!(
            out.contains("let s = self.opt.as_ref()?;"),
            "reference init must become .as_ref()?: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn let_else_ref_mut_init_refused() {
        let input =
            "fn f(o: &mut Option<Vec<u8>>) -> Option<()> { let Some(v) = &mut *o else { return None }; v.push(1); Some(()) }";
        let out = propose_fix("question_mark", input);
        assert!(
            out.is_none(),
            "&mut init would be reborrowed to & by .as_ref() — must refuse: {out:?}"
        );
    }

    #[test]
    fn let_else_other_divergence_untouched() {
        let panic = "fn f(x: Option<u8>) -> u8 { let Some(v) = x else { panic!() }; v }";
        assert!(
            propose_fix("question_mark", panic).is_none(),
            "panic! else-block is not the ? shape"
        );
        let err = "fn f(r: Result<u8, E>) -> Result<u8, E> { let Ok(v) = r else { return Err(E0) }; Ok(v + 1) }";
        assert!(
            propose_fix("question_mark", err).is_none(),
            "Ok-pattern / Err return is outside the Option-only scope"
        );
    }

    #[test]
    fn let_else_nested_pattern_untouched() {
        let input =
            "fn f(p: Option<(u8, u8)>) -> Option<u8> { let Some((a, b)) = p else { return None }; Some(a + b) }";
        let out = propose_fix("question_mark", input);
        assert!(
            out.is_none(),
            "nested tuple pattern is outside the v1 ident-only shape: {out:?}"
        );
    }

    #[test]
    fn let_else_interior_comment_refused() {
        let input =
            "fn f(x: Option<u8>) -> Option<u8> { let Some(v) = /* why */ x else { return None }; Some(v + 1) }";
        let out = propose_fix("question_mark", input);
        assert!(
            out.is_none(),
            "a comment that the splice would drop must refuse: {out:?}"
        );
    }

    #[test]
    fn let_else_non_postfix_safe_init_gets_parens() {
        // Block-like inits are GRAMMAR-FORCED into parens in let-else (syn
        // and rustc both refuse the bare form — probed 2026-10-14), so the
        // non-postfix-safe class arrives as `(match ..)`; the operand law
        // strips the Paren wrapper and re-parenthesizes the inner match.
        let input =
            "fn f(c: bool, x: Option<u8>, o: Option<u8>) -> Option<u8> { let Some(v) = (match c { true => x, false => o }) else { return None }; Some(v + 1) }";
        let out = propose_fix("question_mark", input).expect("matched");
        assert!(
            out.contains("let v = (match c") && out.contains(")?;"),
            "non-postfix-safe init must be parenthesized: {out}"
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

//! `clippy::vec_init_then_push` — `let mut v = Vec::new(); v.push(a);
//! v.push(b);` → `let v = vec![a, b];`.
//!
//! On by default in clippy 1.98 (probe crate, 2026-09-13); real clippy's
//! suggestion is help-only (`consider using the vec![] macro` — `--fix`
//! applied 0/3 even on the plain single-push shape), which is the
//! `MaybeIncorrect` divergence this proposer narrows: the rewrite below is
//! gated on everything parse-visible that could make it wrong.
//!
//! Probe-mapped trigger surface (rustc 1.98.1, 2026-09-13):
//! - fires on `Vec::new()` AND `vec![]` inits, with or without a type
//!   annotation, INCLUDING pushes carrying attributes (probe shape V — the
//!   `#[cfg(feature)]`-gated push of the 089-T3 usage artifact fires too);
//! - the divergence class: a `vec![]` literal cannot carry cfg'd/gated
//!   elements, so an attributed push is REFUSED here while real clippy
//!   still fires (the intake's documented divergence — the correct real-world
//!   fix for that shape is a comment-guarded `allow`).
//!
//! Guards (everything parse-visible that could make the rewrite wrong):
//! - the local must be a plain `let mut v` (annotation kept verbatim), init
//!   `Vec::new()` or empty `vec![]`, no attrs, no let-else;
//! - the pushes must be CONSECUTIVE semicolon statements immediately after
//!   the init, each a bare `v.push(<expr>);` with no attributes — any
//!   attributed/cfg'd push refuses (the divergence class above);
//! - `v` must have NO later mutable use in the same block: no method call on
//!   `v` of ANY kind (conservative — reads like `v.len()` are missed fires,
//!   never wrong fires), no assignment/compound-assign target starting at
//!   `v`, no `&mut` mentioning `v`. Dropping the `mut` is then safe: the
//!   pushes (the only mutable borrowers) are consumed by the rewrite;
//! - no comments inside the replaced span (they would be displaced).
//!
//! Semantics: `vec![a, b]` evaluates its elements left-to-right, exactly
//! as the consecutive pushes did. Identical by construction.

use super::helpers::drops_interior_comment;
use super::{SourceEdit, src};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Block, Expr, File, Local, Pat, Stmt};

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
    fn visit_block(&mut self, b: &'ast Block) {
        syn::visit::visit_block(self, b);
        if let Some(edit) = self.try_collapse(b) {
            self.edits.push(edit);
        }
    }
}

impl Collector<'_> {
    fn try_collapse(&self, b: &Block) -> Option<SourceEdit> {
        let stmts = &b.stmts;
        for i in 0..stmts.len() {
            let Stmt::Local(local) = &stmts[i] else {
                continue;
            };
            let Some((name, ann)) = self.let_shape(local) else {
                continue;
            };
            // The consecutive push run immediately after the init.
            let mut values: Vec<String> = Vec::new();
            let mut j = i + 1;
            while j < stmts.len() {
                let Some((arg, plain)) = self.push_shape(&stmts[j], &name) else {
                    break;
                };
                if !plain {
                    // Attributed push (the cfg-gate divergence) — refuse the
                    // WHOLE site, not just this push.
                    values.clear();
                    break;
                }
                values.push(arg);
                j += 1;
            }
            if values.is_empty() {
                continue;
            }
            // No later mutable use of `v` in this block — stmts after the
            // run (the tail expression IS the last `Stmt::Expr(_, None)`,
            // so the walk covers it).
            if stmts[j..].iter().any(|s| stmt_mentions_mutably(s, &name)) {
                continue;
            }
            let span_start = local.span().byte_range().start;
            let span_end = stmts[j - 1].span().byte_range().end;
            // Whole-span replacement — any interior comment would be
            // displaced (the manual_slice_fill guard class).
            if drops_interior_comment(self.source, span_start..span_end, &[]) {
                continue;
            }
            let replacement = format!("let {name}{ann} = vec![{}];", values.join(", "));
            return Some(SourceEdit {
                start: span_start,
                end: span_end,
                replacement,
            });
        }
        None
    }

    /// `(name, annotation)` of a plain `let mut v [: Ty] = Vec::new();` /
    /// `= vec![];` local — the only init shapes the rewrite folds.
    fn let_shape(&self, local: &Local) -> Option<(String, String)> {
        if !local.attrs.is_empty() {
            return None;
        }
        let init = local.init.as_ref()?;
        if init.diverge.is_some() {
            return None; // let-else
        }
        let (pat, ann) = match &local.pat {
            // `let v: Ty` wraps the ident in Pat::Type.
            Pat::Type(t) => {
                let Pat::Ident(p) = &*t.pat else {
                    return None;
                };
                if p.by_ref.is_some() || p.subpat.is_some() {
                    return None;
                }
                let ty = src(self.source, &t.ty)?;
                (p, format!(": {ty}"))
            }
            Pat::Ident(p) => {
                if p.by_ref.is_some() || p.subpat.is_some() {
                    return None;
                }
                (p, String::new())
            }
            _ => return None,
        };
        // The binding MUST be `mut` — `let v = Vec::new(); v.push(..)` does
        // not compile, so requiring it costs nothing and proves we are
        // looking at a real mutation slot.
        pat.mutability?;
        let init = &*init.expr;
        if !is_vec_new_or_empty_macro(init) {
            return None;
        }
        Some((pat.ident.to_string(), ann))
    }

    /// `(argument source, attribute-free)` if the statement is
    /// `<name>.push(<arg>);` — `None` if it is not a push at all (run end).
    fn push_shape(&self, s: &Stmt, name: &str) -> Option<(String, bool)> {
        let Stmt::Expr(Expr::MethodCall(m), Some(_)) = s else {
            return None;
        };
        if m.method != "push" || m.args.len() != 1 || m.turbofish.is_some() {
            return None;
        }
        let Expr::Path(p) = &*m.receiver else {
            return None;
        };
        if p.path.get_ident().is_none_or(|id| id != name) {
            return None;
        }
        let arg = src(self.source, &m.args[0])?;
        Some((arg, m.attrs.is_empty()))
    }
}

/// `Vec::new()` (bare two-segment path, no args) or an EMPTY `vec![]` macro —
/// attribute-carrying inits are refused (they cannot fold into the literal).
fn is_vec_new_or_empty_macro(e: &Expr) -> bool {
    match e {
        Expr::Call(c) => {
            if !c.attrs.is_empty() || !c.args.is_empty() {
                return false;
            }
            let Expr::Path(p) = &*c.func else {
                return false;
            };
            p.path.segments.len() == 2
                && p.path.segments[0].ident == "Vec"
                && p.path.segments[1].ident == "new"
                && p.path.segments.iter().all(|s| s.arguments.is_none())
        }
        Expr::Macro(m) => {
            m.attrs.is_empty() && m.mac.path.is_ident("vec") && m.mac.tokens.is_empty()
        }
        _ => false,
    }
}

/// Conservative later-mutable-use detector: ANY method call on `name`, any
/// assignment whose LHS starts at `name`, any `&mut` mentioning `name`.
fn stmt_mentions_mutably(s: &Stmt, name: &str) -> bool {
    let mut v = Seeker { name, found: false };
    v.visit_stmt(s);
    v.found
}

/// Does the expression START at a bare path `name` (the LHS of an
/// assignment — `v = ..`, `v[0] = ..`, `v.x = ..`)?
fn starts_with_ident(e: &Expr, name: &str) -> bool {
    match e {
        Expr::Path(p) => p.path.get_ident().is_some_and(|id| id == name),
        Expr::Index(i) => starts_with_ident(&i.expr, name),
        Expr::Field(f) => starts_with_ident(&f.base, name),
        _ => false,
    }
}

struct Seeker<'a> {
    name: &'a str,
    found: bool,
}

impl<'ast> Visit<'ast> for Seeker<'_> {
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if let Expr::Path(p) = &*m.receiver {
            if p.path.get_ident().is_some_and(|id| id == self.name) {
                self.found = true;
            }
        }
        syn::visit::visit_expr_method_call(self, m);
    }
    fn visit_expr_assign(&mut self, a: &'ast syn::ExprAssign) {
        if starts_with_ident(&a.left, self.name) {
            self.found = true;
        }
        syn::visit::visit_expr_assign(self, a);
    }
    fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
        // Compound assignment (`+=` etc.) is a binary op in syn.
        if matches!(
            b.op,
            syn::BinOp::AddAssign(_)
                | syn::BinOp::SubAssign(_)
                | syn::BinOp::MulAssign(_)
                | syn::BinOp::DivAssign(_)
                | syn::BinOp::RemAssign(_)
                | syn::BinOp::BitXorAssign(_)
                | syn::BinOp::BitAndAssign(_)
                | syn::BinOp::BitOrAssign(_)
                | syn::BinOp::ShlAssign(_)
                | syn::BinOp::ShrAssign(_)
        ) && starts_with_ident(&b.left, self.name)
        {
            self.found = true;
        }
        syn::visit::visit_expr_binary(self, b);
    }
    fn visit_expr_reference(&mut self, r: &'ast syn::ExprReference) {
        if r.mutability.is_some() {
            // Under `&mut`, ANY mention of the binding matters (`&mut v`,
            // `&mut v.field`, `&mut v[0]`) — a plain ident-mention walk.
            let mut inner = MentionSeeker {
                name: self.name,
                found: false,
            };
            syn::visit::Visit::visit_expr(&mut inner, &r.expr);
            self.found |= inner.found;
        }
        syn::visit::visit_expr_reference(self, r);
    }
}

/// Plain path-mention walker (used ONLY under `&mut` subtrees, where any
/// mention of the binding is a mutable borrow of it).
struct MentionSeeker<'a> {
    name: &'a str,
    found: bool,
}

impl<'ast> Visit<'ast> for MentionSeeker<'_> {
    fn visit_path(&mut self, p: &'ast syn::Path) {
        if p.get_ident().is_some_and(|id| id == self.name) {
            self.found = true;
        }
        syn::visit::visit_path(self, p);
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn collapses_two_pushes() {
        let input = "fn build() -> Vec<u8> {\n    let mut v = Vec::new();\n    v.push(1);\n    v.push(2);\n    v\n}";
        let out = propose_fix("vec_init_then_push", input).expect("matched");
        assert!(out.contains("let v = vec![1, 2];"), "no collapse: {out}");
        assert!(!out.contains("push"), "push remains: {out}");
        assert!(!out.contains("mut"), "mut remains: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn single_push_fires() {
        // Probe shape E: one push is enough (clippy fires at 1).
        let input = "fn f() -> Vec<i32> { let mut v = Vec::new(); v.push(1); v }";
        let out = propose_fix("vec_init_then_push", input).expect("matched");
        assert!(out.contains("let v = vec![1];"), "no collapse: {out}");
    }

    #[test]
    fn empty_vec_macro_init_fires() {
        // Probe shape T: `vec![]` init + pushes.
        let input = "fn f() -> Vec<i32> { let mut v = vec![]; v.push(1); v.push(2); v }";
        let out = propose_fix("vec_init_then_push", input).expect("matched");
        assert!(out.contains("let v = vec![1, 2];"), "no collapse: {out}");
    }

    #[test]
    fn type_annotation_kept() {
        // Probe shape U: the annotation survives verbatim.
        let input = "fn f() -> Vec<u8> { let mut v: Vec<u8> = Vec::new(); v.push(1); v }";
        let out = propose_fix("vec_init_then_push", input).expect("matched");
        assert!(
            out.contains("let v: Vec<u8> = vec![1];"),
            "annotation lost: {out}"
        );
    }

    #[test]
    fn later_push_refuses() {
        // A later push needs `mut` — the collapse would break the build.
        let input = "fn f(n: i32) -> Vec<i32> { let mut v = Vec::new(); v.push(1); if n > 0 { } v.push(2); v }";
        let out = propose_fix("vec_init_then_push", input);
        assert!(out.is_none(), "later push must refuse: {out:?}");
    }

    #[test]
    fn later_mutation_refuses() {
        let input =
            "fn f(v0: Vec<i32>) -> Vec<i32> { let mut v = Vec::new(); v.push(1); v.extend(v0); v }";
        let out = propose_fix("vec_init_then_push", input);
        assert!(out.is_none(), "later extend must refuse: {out:?}");
    }

    #[test]
    fn later_assign_refuses() {
        let input = "fn f() -> Vec<i32> { let mut v = Vec::new(); v.push(1); v = Vec::new(); v }";
        let out = propose_fix("vec_init_then_push", input);
        assert!(out.is_none(), "later assign must refuse: {out:?}");
    }

    #[test]
    fn later_ref_mut_refuses() {
        let input = "fn take(v: &mut Vec<i32>) {}\nfn f() -> Vec<i32> { let mut v = Vec::new(); v.push(1); take(&mut v); v }";
        let out = propose_fix("vec_init_then_push", input);
        assert!(out.is_none(), "later &mut must refuse: {out:?}");
    }

    #[test]
    fn attributed_push_refuses() {
        // Probe shape V / the 089-T3 usage artifact: cfg-gated pushes fire
        // in real clippy but a vec![] literal cannot carry them — the
        // documented divergence. Correct real-world fix there: allow.
        let input = "fn f() -> Vec<i32> { let mut v = Vec::new(); #[cfg(feature = \"never\")] v.push(1); v.push(2); v }";
        let out = propose_fix("vec_init_then_push", input);
        assert!(out.is_none(), "attributed push must refuse: {out:?}");
    }

    #[test]
    fn non_consecutive_push_refuses() {
        // The run stops at the first non-push stmt; the later-use guard then
        // sees v.push(2) and refuses — mut would still be needed.
        let input = "fn f(log: &mut usize) -> Vec<i32> { let mut v = Vec::new(); v.push(1); *log += 1; v.push(2); v }";
        let out = propose_fix("vec_init_then_push", input);
        assert!(out.is_none(), "split run must refuse: {out:?}");
    }

    #[test]
    fn method_call_tail_refuses() {
        // v.len() after the run is a method call ON v — conservative
        // refusal (a read, but parse-only cannot tell a mutating custom
        // trait method from len).
        let input = "fn f() -> usize { let mut v = Vec::new(); v.push(1); v.len() }";
        let out = propose_fix("vec_init_then_push", input);
        assert!(out.is_none(), "method-call tail must refuse: {out:?}");
    }

    #[test]
    fn mid_read_refuses() {
        // Any later v mention via a method call refuses — conservative.
        let input = "fn f() -> Vec<i32> { let mut v = Vec::new(); v.push(1); let n = v.len(); let _ = n; v }";
        let out = propose_fix("vec_init_then_push", input);
        assert!(out.is_none(), "mid read must refuse: {out:?}");
    }

    #[test]
    fn non_empty_vec_macro_init_refuses() {
        // Only the EMPTY vec![] folds — vec![1] + push needs token surgery.
        let input = "fn f() -> Vec<i32> { let mut v = vec![1]; v.push(2); v }";
        let out = propose_fix("vec_init_then_push", input);
        assert!(out.is_none(), "non-empty macro init must refuse: {out:?}");
    }

    #[test]
    fn interior_comment_refuses() {
        let input = "fn f() -> Vec<i32> { let mut v = Vec::new(); /* seed */ v.push(1); v }";
        let out = propose_fix("vec_init_then_push", input);
        assert!(out.is_none(), "interior comment must refuse: {out:?}");
    }

    #[test]
    fn push_values_preserved_verbatim() {
        let input = "fn f(x: i32) -> Vec<i32> { let mut v = Vec::new(); v.push(x * 2 + 1); v }";
        let out = propose_fix("vec_init_then_push", input).expect("matched");
        assert!(out.contains("vec![x * 2 + 1]"), "value mangled: {out}");
    }
}

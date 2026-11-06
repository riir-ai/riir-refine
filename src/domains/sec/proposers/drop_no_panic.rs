//! `drop-impl-no-panic` — no panicking calls inside `Drop::drop` glue
//! (`.unwrap()`/`.expect(..)` → `.unwrap_or_default()`).
//!
//! A panic inside drop glue while unwinding is a **double-panic abort** —
//! the process dies without running remaining destructors. Drop glue must
//! degrade, never unwrap. `clippy::unwrap_used` (restriction) fires on the
//! same spans but offers NO fix; this rule is drop-glue-scoped WITH one —
//! not an equivalent-fix duplication (the T0.4 boundary's exact line).
//!
//! ## Gates
//! - **`impl Drop for X` → `fn drop` only**: the rewrite is scoped to drop
//!   glue; general unwrap discipline is clippy's restriction surface.
//! - **`.unwrap()` / `.expect("...")` method calls** (no args / one arg):
//!   replaced with `.unwrap_or_default()` on the same receiver. `panic!`,
//!   indexing, arithmetic stay out — no bounded fix.
//! - **No caller gate needed**: the receiver expression is preserved
//!   verbatim; only the method segment changes.
//!
//! ## Honest caveats
//! - `unwrap_or_default` needs `T: Default` — parse-only cannot prove it;
//!   the `--fix --verify` compile gate reverts non-Default receivers.
//! - A panic becomes a default VALUE: the drop path silently proceeds. That
//!   is the drop contract (degrade, never abort); the `.expect` message is
//!   dropped with the panic.

use syn::spanned::Spanned;
use syn::{Expr, ExprCall, ExprMethodCall, File};

use super::{src, SourceEdit};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    for item in &file.items {
        if let syn::Item::Impl(imp) = item {
            if !is_drop_impl(imp) {
                continue;
            }
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    if m.sig.ident == "drop" {
                        collect_in_block(source, &m.block, &mut edits);
                    }
                }
            }
        }
    }
    edits
}

fn is_drop_impl(imp: &syn::ItemImpl) -> bool {
    let Some((_, path, _)) = &imp.trait_ else {
        return false;
    };
    path.segments.last().is_some_and(|s| s.ident == "Drop")
}

fn collect_in_block(source: &str, block: &syn::Block, edits: &mut Vec<SourceEdit>) {
    let mut walker = Walker { source, edits };
    walker.visit_block(block);
}

struct Walker<'a> {
    source: &'a str,
    edits: &'a mut Vec<SourceEdit>,
}

impl Walker<'_> {
    fn visit_block(&mut self, block: &syn::Block) {
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &syn::Stmt) {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    self.visit_expr(&init.expr);
                }
            }
            syn::Stmt::Item(item) => self.visit_nested_item(item),
            syn::Stmt::Expr(e, _) => self.visit_expr(e),
            syn::Stmt::Macro(_) => {}
        }
    }

    fn visit_nested_item(&mut self, item: &syn::Item) {
        if let syn::Item::Fn(f) = item {
            self.visit_block(&f.block);
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::MethodCall(call) => {
                self.check_method_call(call);
                self.visit_expr(&call.receiver);
            }
            Expr::Call(ExprCall { func, args, .. }) => {
                // fn() calls: walk args for nested method calls.
                if let Expr::Path(_) = func.as_ref() {
                    for a in args {
                        self.visit_expr(a);
                    }
                } else {
                    self.visit_expr(func);
                    for a in args {
                        self.visit_expr(a);
                    }
                }
            }
            Expr::Match(m) => {
                self.visit_expr(&m.expr);
                for arm in &m.arms {
                    if let Some((_, guard)) = &arm.guard {
                        self.visit_expr(guard);
                    }
                    self.visit_expr(&arm.body);
                }
            }
            Expr::If(i) => {
                self.visit_expr(&i.cond);
                self.visit_block(&i.then_branch);
                if let Some((_, else_b)) = &i.else_branch {
                    self.visit_expr(else_b);
                }
            }
            Expr::Block(b) => self.visit_block(&b.block),
            Expr::Unary(u) => self.visit_expr(&u.expr),
            Expr::Binary(b) => {
                self.visit_expr(&b.left);
                self.visit_expr(&b.right);
            }
            Expr::Paren(p) => self.visit_expr(&p.expr),
            Expr::Field(f) => self.visit_expr(&f.base),
            Expr::Index(ix) => {
                self.visit_expr(&ix.expr);
                self.visit_expr(&ix.index);
            }
            Expr::Closure(c) => self.visit_expr(&c.body),
            _ => {}
        }
    }

    fn check_method_call(&mut self, call: &ExprMethodCall) {
        let name = call.method.to_string();
        if name != "unwrap" && name != "expect" {
            return;
        }
        let Some(recv) = src(self.source, &call.receiver) else {
            return;
        };
        // Preserve any turbofish-free simple shape: receiver + default.
        let replacement = format!("{recv}.unwrap_or_default()");
        let r = call.span().byte_range();
        self.edits.push(SourceEdit {
            start: r.start,
            end: r.end,
            replacement,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn applied(src: &str) -> (String, usize) {
        let file = syn::parse_file(src).expect("parse");
        let edits = collect_edits(src, &file);
        let n = edits.len();
        (super::super::apply_edits(src, edits), n)
    }

    const BUGGY: &str = "pub struct Conn { pub handle: Option<u32> }\n\nimpl Drop for Conn {\n    fn drop(&mut self) {\n        let h = self.handle.unwrap();\n        println!(\"close {h}\");\n    }\n}\n";

    #[test]
    fn rewrites_unwrap_in_drop() {
        let (out, n) = applied(BUGGY);
        assert_eq!(n, 1);
        assert!(out.contains("self.handle.unwrap_or_default()"), "{out}");
    }

    #[test]
    fn rewrites_expect_dropping_message() {
        let src = "pub struct S { pub h: Option<u32> }\nimpl Drop for S {\n    fn drop(&mut self) { let _ = self.h.expect(\"boom\"); }\n}\n";
        let (out, n) = applied(src);
        assert_eq!(n, 1);
        assert!(out.contains("self.h.unwrap_or_default()"), "{out}");
    }

    #[test]
    fn leaves_unwrap_outside_drop_alone() {
        let src = "pub struct S { pub h: Option<u32> }\nimpl S {\n    pub fn get(&self) -> u32 { self.h.unwrap() }\n}\n";
        let file = syn::parse_file(src).unwrap();
        assert_eq!(
            collect_edits(src, &file),
            [] as [crate::edit_emitter::SourceEdit; 0]
        );
    }

    #[test]
    fn leaves_other_impl_drop_methods_alone() {
        // A method named `drop` on a NON-Drop impl is not drop glue.
        let src = "pub struct S;\nimpl S {\n    pub fn drop(&mut self) { let _ = self.helper().unwrap(); }\n    fn helper(&self) -> Option<u32> { None }\n}\n";
        let file = syn::parse_file(src).unwrap();
        assert_eq!(
            collect_edits(src, &file),
            [] as [crate::edit_emitter::SourceEdit; 0]
        );
    }

    #[test]
    fn catches_nested_unwrap_in_match_arm() {
        let src = "pub struct S { pub h: Option<u32> }\nimpl Drop for S {\n    fn drop(&mut self) {\n        match self.h {\n            Some(v) => println!(\"{v}\"),\n            None => { let _ = self.other().unwrap(); }\n        }\n    }\n    fn other(&self) -> Option<u32> { None }\n}\n";
        let (_, n) = applied(src);
        assert_eq!(n, 1, "nested unwrap in a match arm is caught");
    }
}

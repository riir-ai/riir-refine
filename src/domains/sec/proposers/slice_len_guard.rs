//! `slice-from-raw-parts-len-guard` — guard the `len == 0` case before
//! `slice::from_raw_parts` (ptr must be non-null EVEN when len is zero).
//!
//! The documented safety contract of `from_raw_parts` requires `ptr` to be
//! non-null **even when `len == 0`** — a null + zero-len call that "feels
//! empty" is instant UB. C callers passing `(NULL, 0)` for "no data" are the
//! canonical trap. The bounded fix wraps the call in an `if len == 0`
//! expression returning an empty slice — signature-preserving, additive.
//!
//! ## Gates
//! - **Path calls named `from_raw_parts` / `from_raw_parts_mut`** (any path
//!   prefix: `std::slice::…`, `core::slice::…`, bare import).
//! - **Len is a bare PATH (a runtime value)**: a literal len is deliberate —
//!   literal `0` is already-broken differently, a non-zero literal cannot
//!   hit the UB — both skip.
//! - **No existing guard**: the file text containing `<len> == 0` or
//!   `<len>==0` means the empty case is already handled (or the ident is
//!   guarded elsewhere in the file) — skip. Whole-file scan is MORE
//!   conservative than a per-fn scan; in sec, conservative is correct.
//! - **No caller gate needed**: expression-local wrap, the surrounding
//!   context (let/tail/arg position) is preserved by construction.
//!
//! ## Honest caveats
//! - The guard fixes ONLY the `len == 0 && ptr.is_null()` UB class. For
//!   `len > 0`, ptr non-null/aligned stays the caller's contract.
//! - `&[]` / `&mut []` unify with the call's slice type by inference in
//!   every position where the original call compiled (let, return, arg) —
//!   the compile gate reverts exotic contexts.

use syn::spanned::Spanned;
use syn::{Expr, ExprCall, File};

use super::{SourceEdit, src};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut walker = Walker {
        source,
        edits: Vec::new(),
    };
    walker.visit_items(&file.items);
    walker.edits
}

struct Walker<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl Walker<'_> {
    fn visit_items(&mut self, items: &[syn::Item]) {
        for item in items {
            match item {
                syn::Item::Fn(f) => self.visit_block(&f.block),
                syn::Item::Mod(m) => {
                    if let Some((_, inner)) = &m.content {
                        self.visit_items(inner);
                    }
                }
                syn::Item::Impl(imp) => {
                    for ii in &imp.items {
                        if let syn::ImplItem::Fn(m) = ii {
                            self.visit_block(&m.block);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn visit_block(&mut self, block: &syn::Block) {
        for stmt in &block.stmts {
            match stmt {
                syn::Stmt::Local(l) => {
                    if let Some(init) = &l.init {
                        self.visit_expr(&init.expr);
                    }
                }
                syn::Stmt::Expr(e, _) => self.visit_expr(e),
                _ => {}
            }
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::Call(call) = expr {
            if Self::is_from_raw_parts(call) {
                self.maybe_guard(call, expr);
            }
        }
        // Recurse into common shapes for nested calls.
        match expr {
            Expr::Call(c) => {
                self.visit_expr(&c.func);
                for a in &c.args {
                    self.visit_expr(a);
                }
            }
            Expr::MethodCall(m) => self.visit_expr(&m.receiver),
            Expr::Return(r) => {
                if let Some(e) = &r.expr {
                    self.visit_expr(e);
                }
            }
            Expr::Match(m) => {
                self.visit_expr(&m.expr);
                for arm in &m.arms {
                    if let Some((_, g)) = &arm.guard {
                        self.visit_expr(g);
                    }
                    self.visit_expr(&arm.body);
                }
            }
            Expr::If(i) => {
                self.visit_expr(&i.cond);
                self.visit_block(&i.then_branch);
                if let Some((_, e)) = &i.else_branch {
                    self.visit_expr(e);
                }
            }
            Expr::Block(b) => self.visit_block(&b.block),
            Expr::Paren(p) => self.visit_expr(&p.expr),
            Expr::Unary(u) => self.visit_expr(&u.expr),
            Expr::Binary(b) => {
                self.visit_expr(&b.left);
                self.visit_expr(&b.right);
            }
            Expr::Cast(c) => self.visit_expr(&c.expr),
            _ => {}
        }
    }

    fn is_from_raw_parts(call: &ExprCall) -> bool {
        let Expr::Path(p) = call.func.as_ref() else {
            return false;
        };
        p.path
            .segments
            .last()
            .is_some_and(|s| s.ident == "from_raw_parts" || s.ident == "from_raw_parts_mut")
    }

    fn maybe_guard(&mut self, call: &ExprCall, whole: &Expr) {
        if call.args.len() != 2 {
            return;
        }
        let len_arg = &call.args[1];
        let Expr::Path(len_path) = len_arg else {
            return;
        };
        let Some(seg) = len_path.path.segments.last() else {
            return;
        };
        let ident = seg.ident.to_string();

        // Existing guard in the enclosing source? Conservative textual
        // check across the whole file (a same-file guard anywhere near is
        // enough evidence of intent to skip).
        if self.source.contains(&format!("{ident} == 0"))
            || self.source.contains(&format!("{ident}==0"))
        {
            return;
        }
        let Some(call_text) = src(self.source, whole) else {
            return;
        };
        let empty = if call_text.contains("from_raw_parts_mut") {
            "&mut []"
        } else {
            "&[]"
        };
        let replacement = format!("if {ident} == 0 {{ {empty} }} else {{ {call_text} }}");
        let r = whole.span().byte_range();
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

    const BUGGY: &str = "pub unsafe fn as_bytes(buf: *const u8, len: usize) -> &'static [u8] {\n    std::slice::from_raw_parts(buf, len)\n}\n";

    #[test]
    fn wraps_with_len_zero_guard() {
        let (out, n) = applied(BUGGY);
        assert_eq!(n, 1);
        assert!(
            out.contains("if len == 0 { &[] } else { std::slice::from_raw_parts(buf, len) }"),
            "{out}"
        );
    }

    #[test]
    fn uses_mut_empty_for_mut_variant() {
        let src = "pub unsafe fn f(b: *mut u8, n: usize) -> &'static mut [u8] {\n    std::slice::from_raw_parts_mut(b, n)\n}\n";
        let (out, n) = applied(src);
        assert_eq!(n, 1);
        assert!(out.contains("&mut []"), "{out}");
    }

    #[test]
    fn skips_literal_len() {
        let src = "pub unsafe fn f(b: *const u8) -> &'static [u8] {\n    std::slice::from_raw_parts(b, 8)\n}\n";
        let file = syn::parse_file(src).unwrap();
        assert_eq!(
            collect_edits(src, &file),
            [] as [crate::edit_emitter::SourceEdit; 0]
        );
    }

    #[test]
    fn skips_existing_guard() {
        let src = "pub unsafe fn f(b: *const u8, n: usize) -> &'static [u8] {\n    if n == 0 { return &[]; }\n    std::slice::from_raw_parts(b, n)\n}\n";
        let file = syn::parse_file(src).unwrap();
        assert_eq!(
            collect_edits(src, &file),
            [] as [crate::edit_emitter::SourceEdit; 0]
        );
    }

    #[test]
    fn fires_in_let_position() {
        let src = "pub unsafe fn f(b: *const u8, n: usize) {\n    let s: &[u8] = std::slice::from_raw_parts(b, n);\n    println!(\"{}\", s.len());\n}\n";
        let (out, n) = applied(src);
        assert_eq!(n, 1);
        assert!(
            out.contains(
                "let s: &[u8] = if n == 0 { &[] } else { std::slice::from_raw_parts(b, n) };"
            ),
            "{out}"
        );
    }
}

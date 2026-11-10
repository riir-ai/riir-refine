//! `nan-unsafe-sort-comparator` — `partial_cmp(..).unwrap_or(Equal)` (or
//! `.unwrap()`) inside a sort comparator → `total_cmp`.
//!
//! `partial_cmp` returns `None` on NaN, so the comparator orders NaN as
//! equal to EVERYTHING — an inconsistent (non-total) order. Since Rust 1.81
//! the std sorts document that they MAY PANIC on exactly that input
//! ("user-provided comparison function does not correctly implement a total
//! order"), and every older sort silently mis-orders NaN. `total_cmp` is
//! the repair: a real total order, NaN-deterministic.
//!
//! ## Why the rewrite is mechanically safe here (the P8 refinement)
//!
//! The P8 intake assumed the replacement needed operand DIRECTION and float
//! TYPE (the `float_order::{desc,asc,desc_f64,asc_f64}` twin problem). std
//! `total_cmp` dissolves both: `a.partial_cmp(b)` → `a.total_cmp(&b)`
//! preserves the recv/arg order mechanically (direction rides the same
//! positions), and `total_cmp` exists name-identically on `f32` AND `f64`.
//! Residual hazard: a NON-float receiver with `partial_cmp` but no
//! `total_cmp` (e.g. `Option<f32>`) fails E0599 — the `--fix --verify`
//! compile gate auto-reverts it (the `constant-time-secret-compare`
//! precedent). For all non-NaN inputs the rewrite is order-identical.
//!
//! ## Gates (conservative — refuse more than fires)
//!
//! - **Sort context only**: the enclosing fn (any nesting depth) must
//!   contain a `sort_by` / `sort_unstable_by` / `select_nth_unstable_by`
//!   call. Outside a sort, `unwrap_or(Equal)` on `partial_cmp` is a
//!   semantic decision the caller owns (suggest territory, not a fix).
//! - **Exact chain shape**: `<recv>.partial_cmp(<1 arg>)` followed by
//!   exactly one `.unwrap_or(<path ending in Equal>)` or `.unwrap()`.
//!   `unwrap_or` with any other default refuses (a different default is a
//!   semantic decision, never mechanical). Deeper chains rewrite only the
//!   matched prefix — `a.partial_cmp(b).unwrap_or(Equal).reverse()`
//!   becomes `a.total_cmp(&b).reverse()` (correct composition).
//! - **Comment guard**: any comment marker inside the replaced chain span
//!   refuses (the house `drops_interior_comment` discipline — the rewrite
//!   would displace it; kept fragments: the receiver + the argument, both
//!   carried verbatim into the replacement).
//! - **Macro args refuse** (token-stream spans are unreliable). The arg
//!   carries over VERBATIM — `PartialOrd::partial_cmp` pins the trait
//!   signature `(&self, other: &Self)`, so a pre-compiling call site
//!   already has `arg: &Self`, which is exactly what `total_cmp` takes:
//!   the output is the idiomatic human form, no `&` insertion needed.
//!
//! Honest recall limit: a comparator defined in a HELPER fn and passed to
//! `sort_by(helper)` by name is not detected — the sort call and the chain
//! must share the enclosing fn (the intake's spec; precision over recall).

use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, Item};

use super::{SourceEdit, src};
use crate::edit_emitter::comment_marker_positions;

/// The sort-family method names that license the rewrite (the precision
/// guard): a comparator chain is only rewritten where a sort consumes it.
const SORT_METHODS: &[&str] = &["sort_by", "sort_unstable_by", "select_nth_unstable_by"];

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut edits);
    }
    edits
}

fn collect_in_item(source: &str, item: &Item, edits: &mut Vec<SourceEdit>) {
    match item {
        Item::Fn(f) => collect_in_fn(source, &f.block, edits),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    collect_in_fn(source, &m.block, edits);
                }
            }
        }
        Item::Mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, edits);
                }
            }
        }
        _ => {}
    }
}

fn collect_in_fn(source: &str, block: &syn::Block, edits: &mut Vec<SourceEdit>) {
    if !block_calls_sort(block) {
        return;
    }
    let mut walker = NanCmpWalker { source, edits };
    walker.visit_block(block);
}

/// A whole-block scan for any sort-family method call (allocation-free:
/// collects nothing, just a boolean).
fn block_calls_sort(block: &syn::Block) -> bool {
    struct SortScan(bool);
    impl<'ast> syn::visit::Visit<'ast> for SortScan {
        fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
            if SORT_METHODS.iter().any(|m| call.method == m) {
                self.0 = true;
            }
            syn::visit::visit_expr_method_call(self, call);
        }
    }
    let mut scan = SortScan(false);
    scan.visit_block(block);
    scan.0
}

struct NanCmpWalker<'a> {
    source: &'a str,
    edits: &'a mut Vec<SourceEdit>,
}

impl NanCmpWalker<'_> {
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
            syn::Stmt::Item(item) => collect_in_item(self.source, item, self.edits),
            syn::Stmt::Expr(e, _) => self.visit_expr(e),
            syn::Stmt::Macro(_) => {}
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::MethodCall(call) => {
                self.check_terminal_call(call);
                self.visit_expr(&call.receiver);
                for a in &call.args {
                    self.visit_expr(a);
                }
            }
            Expr::Call(call) => {
                self.visit_expr(&call.func);
                for a in &call.args {
                    self.visit_expr(a);
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
            Expr::Unsafe(u) => self.visit_block(&u.block),
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
            // Loop bodies are a primary sort habitat (sort inside `for` over
            // cases/batches); without these arms the walker detects the sort
            // via the whole-block scan but never reaches the site.
            Expr::ForLoop(f) => {
                self.visit_expr(&f.expr);
                self.visit_block(&f.body);
            }
            Expr::While(w) => {
                self.visit_expr(&w.cond);
                self.visit_block(&w.body);
            }
            Expr::Loop(l) => self.visit_block(&l.body),
            Expr::Let(l) => self.visit_expr(&l.expr),
            Expr::Assign(a) => {
                self.visit_expr(&a.left);
                self.visit_expr(&a.right);
            }
            Expr::Return(r) => {
                if let Some(e) = &r.expr {
                    self.visit_expr(e);
                }
            }
            Expr::Try(t) => self.visit_expr(&t.expr),
            Expr::Yield(y) => {
                if let Some(e) = &y.expr {
                    self.visit_expr(e);
                }
            }
            Expr::Await(a) => self.visit_expr(&a.base),
            Expr::Range(r) => {
                if let Some(e) = &r.start {
                    self.visit_expr(e);
                }
                if let Some(e) = &r.end {
                    self.visit_expr(e);
                }
            }
            Expr::Reference(r) => self.visit_expr(&r.expr),
            Expr::Group(g) => self.visit_expr(&g.expr),
            Expr::Repeat(r) => {
                self.visit_expr(&r.expr);
                self.visit_expr(&r.len);
            }
            _ => {}
        }
    }

    /// `call` is the TERMINAL method (`.unwrap_or(..)` / `.unwrap()`);
    /// its receiver must be the `<recv>.partial_cmp(<arg>)` chain.
    fn check_terminal_call(&mut self, call: &ExprMethodCall) {
        let method = call.method.to_string();
        let partial = match call.receiver.as_ref() {
            Expr::MethodCall(p) if p.method == "partial_cmp" => p,
            _ => return,
        };
        match method.as_str() {
            "unwrap" if call.args.is_empty() => {}
            "unwrap_or" if call.args.len() == 1 => {
                // The default must be a path ending in `Equal` — any other
                // default is a semantic decision, never mechanical.
                match call.args.first() {
                    Some(Expr::Path(p))
                        if p.path.segments.last().is_some_and(|s| s.ident == "Equal") => {}
                    _ => return,
                }
            }
            _ => return,
        }
        if partial.args.len() != 1 {
            return;
        }
        let Some(arg) = partial.args.first() else {
            return;
        };
        // Macro args: token-stream spans are unreliable — refuse.
        if matches!(arg, Expr::Macro(_)) || matches!(partial.receiver.as_ref(), Expr::Macro(_)) {
            return;
        }
        // Tuple literals refuse: `total_cmp` is an f32/f64-inherent, NOT a
        // tuple method — `(f32, f32).partial_cmp(..)` (egui-Rect-style
        // multi-field compares) has no tuple `total_cmp` to rewrite to, and
        // the splice is E0599 under `--tests` (found live in seal-game-editor
        // editor-widgets, sorted-by-rect-position test). Parse-only can't see
        // every no-total_cmp type, but a literal tuple IS the common
        // multi-field shape and is detectable — refuse it here.
        if matches!(partial.receiver.as_ref(), Expr::Tuple(_)) || matches!(arg, Expr::Tuple(_)) {
            return;
        }
        let chain_start = byte_start(partial.receiver.as_ref());
        let chain_end = call.span().byte_range().end;
        // Comment guard: the rewrite replaces the whole chain span; a
        // comment inside it would be displaced. Kept fragments are the
        // receiver and the argument (both carried verbatim).
        let kept = [
            partial.receiver.span().byte_range(),
            arg.span().byte_range(),
        ];
        if let Some(chain) = self.source.get(chain_start..chain_end) {
            if comment_marker_positions(chain).iter().any(|&pos| {
                let abs = chain_start + pos;
                !kept.iter().any(|k| k.contains(&abs))
            }) {
                return;
            }
        } else {
            return; // broken span bookkeeping — refuse rather than displace
        }
        let Some(recv_text) = src(self.source, &partial.receiver.span()) else {
            return;
        };
        let Some(arg_text) = src(self.source, &arg.span()) else {
            return;
        };
        // The arg carries over VERBATIM: `PartialOrd::partial_cmp` pins the
        // trait signature `(&self, other: &Self)`, so any pre-compiling call
        // site already has `arg: &Self` — and `total_cmp` is `(&self,
        // other: &Self)` too. No `&` insertion, no parenthesization, and the
        // output is the idiomatic human form. (The only breaks are types
        // with `partial_cmp` but no `total_cmp` — `Option<f32>`, other Ord
        // primitives — all E0599, all auto-reverted by `--verify`.)
        let replacement = format!("{recv_text}.total_cmp({arg_text})");
        self.edits.push(SourceEdit {
            start: chain_start,
            end: chain_end,
            replacement,
        });
    }
}

/// Span start that survives method-call receivers (syn spans on exprs are
/// call-site dependent; the byte range is what we splice against).
fn byte_start(expr: &Expr) -> usize {
    expr.span().byte_range().start
}

#[cfg(test)]
mod tests {
    use super::*;

    fn applied(src_text: &str) -> (String, usize) {
        let file = syn::parse_file(src_text).expect("parse");
        let edits = collect_edits(src_text, &file);
        let n = edits.len();
        (super::super::apply_edits(src_text, edits), n)
    }

    const BUGGY: &str = "pub fn sort_scores(v: &mut [f32]) {\n    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));\n}\n";

    #[test]
    fn rewrites_the_corpus_fixture() {
        let (out, n) = applied(BUGGY);
        assert_eq!(n, 1, "{out}");
        assert!(out.contains("a.total_cmp(b)"), "{out}");
        assert!(!out.contains("partial_cmp"), "{out}");
    }

    #[test]
    fn rewrites_imported_ordering_form() {
        let src_text = "use std::cmp::Ordering;\npub fn f(v: &mut [f32]) {\n    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));\n}\n";
        let (out, n) = applied(src_text);
        assert_eq!(n, 1);
        assert!(out.contains("a.total_cmp(b)"), "{out}");
    }

    #[test]
    fn rewrites_unwrap_form() {
        let src_text = "pub fn f(v: &mut [f64]) {\n    v.sort_unstable_by(|a, b| b.partial_cmp(a).unwrap());\n}\n";
        let (out, n) = applied(src_text);
        assert_eq!(n, 1);
        assert!(out.contains("b.total_cmp(a)"), "{out}");
    }

    #[test]
    fn rewrites_select_nth_form() {
        let src_text = "pub fn f(v: &mut [f32], k: usize) {\n    v.select_nth_unstable_by(k, |a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));\n}\n";
        let (out, n) = applied(src_text);
        assert_eq!(n, 1);
        assert!(out.contains("a.total_cmp(b)"), "{out}");
    }

    #[test]
    fn no_sort_context_refuses() {
        let src_text = "pub fn cmp(a: f32, b: f32) -> std::cmp::Ordering {\n    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)\n}\n";
        let (_, n) = applied(src_text);
        assert_eq!(n, 0, "outside a sort the default is a caller decision");
    }

    #[test]
    fn non_equal_default_refuses() {
        let src_text = "pub fn f(v: &mut [f32]) {\n    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less));\n}\n";
        let (_, n) = applied(src_text);
        assert_eq!(n, 0);
    }

    #[test]
    fn interior_comment_refuses() {
        let src_text = "pub fn f(v: &mut [f32]) {\n    v.sort_by(|a, b| a.partial_cmp(/* mid */ b).unwrap_or(Ordering::Equal));\n}\n";
        let (_, n) = applied(src_text);
        assert_eq!(n, 0, "the rewrite would displace the comment");
    }

    #[test]
    fn prefix_composition_keeps_trailing_methods() {
        let src_text = "pub fn f(v: &mut [f32]) {\n    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal).reverse());\n}\n";
        let (out, n) = applied(src_text);
        assert_eq!(n, 1);
        assert!(
            out.contains("a.total_cmp(b).reverse()"),
            "the trailing method composes: {out}"
        );
    }

    #[test]
    fn field_arg_carried_verbatim() {
        let src_text = "pub fn f(v: &mut [S]) {\n    v.sort_by(|a, b| a.score.partial_cmp(b.score).unwrap_or(Ordering::Equal));\n}\n";
        let (out, n) = applied(src_text);
        assert_eq!(n, 1);
        assert!(out.contains("a.score.total_cmp(b.score)"), "{out}");
    }

    #[test]
    fn helper_fn_comparator_outside_sort_refuses() {
        // The honest recall limit: sort and chain must share the fn.
        let src_text = "pub fn cmp(a: &f32, b: &f32) -> std::cmp::Ordering {\n    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)\n}\npub fn g(v: &mut [f32]) {\n    v.sort_by(cmp);\n}\n";
        let (_, n) = applied(src_text);
        assert_eq!(n, 0);
    }

    #[test]
    fn sort_inside_for_loop_fires() {
        // Loop bodies are a primary sort habitat — the walker must descend
        // (production shape: per-case probe sorting inside `for case in ..`).
        let src_text = "pub fn g(cases: &[Vec<f32>]) {\n    for c in cases {\n        let mut v = c.clone();\n        v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));\n    }\n}\n";
        let (out, n) = applied(src_text);
        assert_eq!(n, 1);
        assert!(out.contains("b.total_cmp(a)"), "{out}");
    }

    #[test]
    fn sort_inside_while_and_loop_fires() {
        let src_text = "pub fn g(v: &mut [f32], w: &mut [f64]) {\n    while !v.is_empty() {\n        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));\n        break;\n    }\n    loop {\n        w.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());\n        break;\n    }\n}\n";
        let (_, n) = applied(src_text);
        assert_eq!(n, 2);
    }

    #[test]
    fn chain_inside_assign_and_try_fires_alongside_a_sort() {
        // The sort-context gate still applies — assign/try/arm shapes only
        // need REACHING once a sort shares the fn (the nested-fn shadow test
        // pins the boundary in the other direction).
        let src_text = "pub fn g(v: &mut [f32]) -> std::io::<()> {\n    v.sort_by(|a, b| b.total_cmp(a));\n    let mut ord = std::cmp::Ordering::Equal;\n    ord = v[0].partial_cmp(&v[1]).unwrap_or(std::cmp::Ordering::Equal);\n    let _ = ord;\n    let checked = Some(v[0].partial_cmp(&v[1]).unwrap_or(std::cmp::Ordering::Equal))?;\n    let _ = checked;\n    Ok(())\n}\n";
        let (out, n) = applied(src_text);
        assert_eq!(n, 2);
        assert!(out.contains("v[0].total_cmp(&v[1])"), "{out}");
    }

    #[test]
    fn macro_operand_refuses() {
        // The gate exists for span reliability — pin it with a fixture.
        let src_text = "macro_rules! f { () => { 1.0f32 } }\npub fn g(v: &mut [f32]) {\n    v.sort_by(|a, b| a.partial_cmp(f!()).unwrap_or(Ordering::Equal));\n}\n";
        let (_, n) = applied(src_text);
        assert_eq!(n, 0, "macro operands have unreliable spans — refuse");
    }

    #[test]
    fn tuple_receiver_refuses() {
        // `total_cmp` is an f32/f64-inherent, NOT a tuple method: a
        // multi-field compare like `(a.top(), a.left()).partial_cmp(..)` has
        // no tuple `total_cmp` to rewrite to — the splice is E0599 under
        // `--tests` (found live in seal-game-editor editor-widgets sorting
        // egui::Rect positions). The literal-tuple shape is parse-detectable.
        let src_text = "pub fn g(v: &mut [(f32, f32)]) {\n    v.sort_by(|a, b| {\n        (a.0, a.1)\n            .partial_cmp(&(b.0, b.1))\n            .unwrap()\n    });\n}\n";
        let (_, n) = applied(src_text);
        assert_eq!(n, 0, "tuples have no total_cmp — refuse");
    }

    #[test]
    fn scalar_receiver_still_fires_beside_tuple_refusal() {
        // Positive control for the tuple guard: the scalar form in the same
        // fn still rewrites.
        let src_text = "pub fn g(v: &mut [(f32, f32)]) {\n    v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));\n}\n";
        let (out, n) = applied(src_text);
        assert_eq!(n, 1);
        assert!(out.contains("a.0.total_cmp(&b.0)"), "{out}");
    }

    #[test]
    fn nested_fn_without_sort_shadows_the_outer_sort() {
        // A chain inside a NESTED fn must consult the nested fn's own body:
        // the outer fn's sort does not license it (the innermost-habitat
        // semantics — collect_in_item re-checks per fn).
        let src_text = "pub fn outer(v: &mut [f32]) {\n    v.sort_by(|a, b| a.total_cmp(b));\n    fn inner(a: &f32, b: &f32) -> std::cmp::Ordering {\n        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)\n    }\n    let _ = inner;\n}\n";
        let (_, n) = applied(src_text);
        assert_eq!(n, 0, "the nested fn has no sort of its own — refuse");
    }

    #[test]
    fn sort_by_key_is_not_a_comparator_habitat() {
        // Keys return K, not Ordering — a chain there is not a comparator.
        let src_text = "pub fn g(v: &mut [f32]) {\n    v.sort_by_key(|x| x.to_bits());\n    let o = 1.0f32.partial_cmp(&2.0).unwrap_or(std::cmp::Ordering::Equal);\n    let _ = o;\n}\n";
        let (_, n) = applied(src_text);
        assert_eq!(n, 0);
    }

    #[test]
    fn corpus_round_trip() {
        use crate::domain::HealDomain;
        use crate::domains::sec::SecDomain;
        let corpus = SecDomain::build_corpus();
        let rule = corpus
            .iter()
            .find(|r| r.rule_id == "nan-unsafe-sort-comparator")
            .expect("rule present");
        let file = syn::parse_file(&rule.buggy_code).expect("buggy parses");
        let edits = collect_edits(&rule.buggy_code, &file);
        assert_eq!(edits.len(), 1);
        let out = super::super::apply_edits(&rule.buggy_code, edits);
        assert_eq!(
            out, rule.fixed_code,
            "the corpus fixed fixture is the exact rewrite"
        );
    }
}

//! `clippy::redundant_closure` — `|x| f(x)` → `f` when the closure body is a
//! single call passing the closure's only parameter as the only argument.
//!
//! Span-preserving (T6.4): the closure's byte range is replaced by the
//! ORIGINAL source text of the callee path.

use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprClosure, File};

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
    fn visit_expr_closure(&mut self, c: &'ast ExprClosure) {
        // Depth-first: inner redundant closures collect their own edits (an
        // outer match's replacement region contains no other bare-id use, so
        // overlap selection cannot drop a live inner fix here).
        syn::visit::visit_expr_closure(self, c);
        if let Some(callee) = simplify_closure(c) {
            if let Some(text) = src(self.source, callee) {
                self.edits.push(edit(c, text));
            }
        }
    }
}

/// If the closure is `|x| f(x)`, return the callee path `f`. Otherwise None.
fn simplify_closure(closure: &ExprClosure) -> Option<&syn::ExprPath> {
    if closure.inputs.len() != 1 {
        return None;
    }
    // The body is `Box<Expr>`; for `|x| f(x)`, the body is the call directly.
    // For `|x| { f(x) }`, the body is an `ExprBlock`. Handle the bare-call form.
    let call_expr = match &*closure.body {
        Expr::Call(_) => &*closure.body,
        Expr::Block(b) if b.block.stmts.len() == 1 => match &b.block.stmts[0] {
            syn::Stmt::Expr(e, None) => e,
            _ => return None,
        },
        _ => return None,
    };
    let Expr::Call(call) = call_expr else {
        return None;
    };
    if call.args.len() != 1 {
        return None;
    }
    let arg = &call.args[0];
    let syn::Pat::Ident(param_pat) = &closure.inputs[0] else {
        return None;
    };
    let arg_ident = match arg {
        Expr::Path(p) => p.path.get_ident(),
        _ => return None,
    };
    let arg_ident = arg_ident?;
    if arg_ident != &param_pat.ident {
        return None;
    }
    match &*call.func {
        Expr::Path(p) => Some(p),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn simplifies_redundant_closure_in_call() {
        let input = "fn sq(x: u32) -> u32 { x * x }\nfn run(v: Vec<u32>) -> Vec<u32> { v.into_iter().map(|n| sq(n)).collect() }";
        let out = propose_fix("redundant_closure", input).expect("matched");
        assert!(out.contains(".map(sq)"), "closure not replaced by bare path: {out}");
        assert!(out.contains("fn sq(x: u32) -> u32 { x * x }"), "unrelated fn must be byte-identical: {out}");
    }

    #[test]
    fn leaves_non_redundant_closure() {
        let input = "fn go() { let f = |x: i32| x + 1; }";
        let out = propose_fix("redundant_closure", input);
        assert!(out.is_none(), "non-redundant closure touched: {out:?}");
    }

    #[test]
    fn leaves_two_arg_closure() {
        let input = "fn go() { let f = |x: i32, y: i32| add(x, y); }";
        let out = propose_fix("redundant_closure", input);
        assert!(out.is_none(), "two-param closure touched: {out:?}");
    }

    #[test]
    fn leaves_method_call_closure() {
        let input = "fn go() { let f = |x: i32| x.abs(); }";
        let out = propose_fix("redundant_closure", input);
        assert!(out.is_none(), "method-call closure touched: {out:?}");
    }

    #[test]
    fn qualified_path_callee_preserved() {
        let input = "fn run(v: Vec<u32>) -> Vec<u32> { v.into_iter().map(|n| u32::from(n)).collect() }";
        let out = propose_fix("redundant_closure", input).expect("matched");
        assert!(out.contains("map(u32::from)"), "{out}");
    }
}

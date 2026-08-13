//! `clippy::redundant_closure` — `|x| f(x)` → `f` when the closure body is a
//! single call passing the closure's only parameter as the only argument.

use syn::{Expr, ExprClosure, File};

pub(crate) fn rewrite(file: &mut File) -> bool {
    use syn::visit_mut::VisitMut;
    struct Visitor {
        changed: bool,
    }
    impl VisitMut for Visitor {
        fn visit_expr_mut(&mut self, e: &mut Expr) {
            syn::visit_mut::visit_expr_mut(self, e);
            if let Expr::Closure(closure) = e {
                if let Some(replacement) = simplify_closure(closure) {
                    *e = replacement;
                    self.changed = true;
                }
            }
        }
    }
    let mut v = Visitor { changed: false };
    syn::visit_mut::visit_file_mut(&mut v, file);
    v.changed
}

/// If the closure is `|x| f(x)`, return `Expr::Path(f)`. Otherwise None.
fn simplify_closure(closure: &ExprClosure) -> Option<Expr> {
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
    let callee = match &*call.func {
        Expr::Path(p) => p.clone(),
        _ => return None,
    };
    Some(Expr::Path(callee))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn propose(input: &str) -> Option<String> {
        let mut file = syn::parse_file(input).ok()?;
        if rewrite(&mut file) {
            Some(super::super::quote_file(&file))
        } else {
            None
        }
    }

    #[test]
    fn simplifies_redundant_closure_in_call() {
        let input = "fn sq(x: u32) -> u32 { x * x }\nfn run(v: Vec<u32>) -> Vec<u32> { v.into_iter().map(|n| sq(n)).collect() }";
        let out = propose(input).expect("matched");
        // The closure should be gone; `sq` should appear as a bare path.
        assert!(out.contains("sq"), "callee lost: {out}");
    }

    #[test]
    fn leaves_non_redundant_closure() {
        let input = "fn go() { let f = |x: i32| x + 1; }";
        let out = propose(input);
        assert!(out.is_none(), "non-redundant closure touched: {out:?}");
    }

    #[test]
    fn leaves_two_arg_closure() {
        let input = "fn go() { let f = |x: i32, y: i32| add(x, y); }";
        let out = propose(input);
        assert!(out.is_none(), "two-param closure touched: {out:?}");
    }

    #[test]
    fn leaves_method_call_closure() {
        let input = "fn go() { let f = |x: i32| x.abs(); }";
        let out = propose(input);
        assert!(out.is_none(), "method-call closure touched: {out:?}");
    }
}

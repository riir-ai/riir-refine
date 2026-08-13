//! `clippy::needless_borrow` — `f(&x)` → `f(x)` when `x` is a bare path.

use syn::{Expr, File};

pub(crate) fn rewrite(file: &mut File) -> bool {
    use syn::visit_mut::VisitMut;
    struct Visitor {
        changed: bool,
    }
    impl VisitMut for Visitor {
        fn visit_expr_mut(&mut self, e: &mut Expr) {
            syn::visit_mut::visit_expr_mut(self, e);
            if let Expr::Call(call) = e {
                for arg in &mut call.args {
                    if let Expr::Reference(r) = arg {
                        if r.mutability.is_none() && matches!(&*r.expr, Expr::Path(..)) {
                            let inner = &*r.expr;
                            let cloned = inner.clone();
                            *arg = cloned;
                            self.changed = true;
                        }
                    }
                }
            }
        }
    }
    let mut v = Visitor { changed: false };
    syn::visit_mut::visit_file_mut(&mut v, file);
    v.changed
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
    fn strips_borrow_on_path_arg() {
        let input = "fn show(s: &str) -> usize { s.len() }\nfn go(s: &str) -> usize { show(&s) }";
        let out = propose(input).expect("matched");
        // The `&s` in the CALL SITE should become `s`. The function signature
        // `s: &str` keeps its `&` (legitimate — that's the param type).
        // Look for `show (s)` (the call) + verify there's no `& s` after it.
        assert!(out.contains("show ("), "call not present: {out}");
        assert!(!out.contains("& s )"), "borrow still in call: {out}");
    }

    #[test]
    fn leaves_mut_borrow() {
        let input = "fn go(v: &mut Vec<u8>) { foo(&mut v) }";
        let out = propose(input);
        assert!(out.is_none(), "mut borrow touched: {out:?}");
    }

    #[test]
    fn leaves_borrow_of_complex_expr() {
        let input = "fn go() { foo(&(x + 1)) }";
        let out = propose(input);
        assert!(out.is_none(), "complex borrow touched: {out:?}");
    }
}

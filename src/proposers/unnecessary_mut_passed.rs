//! `clippy::unnecessary_mut_passed` — `f(&mut v)` → `f(&v)` + drop `mut`.

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
                        if r.mutability.is_some() && matches!(&*r.expr, Expr::Path(..)) {
                            r.mutability = None;
                            self.changed = true;
                        }
                    }
                }
            }
        }
    }
    let mut v = Visitor { changed: false };
    syn::visit_mut::visit_file_mut(&mut v, file);

    // Second pass: drop `mut` from `fn` parameter patterns.
    if v.changed {
        for item in &mut file.items {
            if let syn::Item::Fn(fn_item) = item {
                for input in &mut fn_item.sig.inputs {
                    if let syn::FnArg::Typed(pat) = input {
                        if let syn::Pat::Ident(pi) = &mut *pat.pat {
                            if pi.mutability.is_some() {
                                pi.mutability = None;
                            }
                        }
                    }
                }
            }
        }
    }
    v.changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn propose(input: &str) -> Option<String> {
        let mut file = syn::parse_file(input).ok()?;
        if rewrite(&mut file) {
            Some(crate::proposers::quote_file(&file))
        } else {
            None
        }
    }

    #[test]
    fn drops_mut_from_call_and_binding() {
        let input = "fn count(v: &Vec<u8>) -> usize { v.len() }\nfn go(mut v: Vec<u8>) -> usize { count(&mut v) }";
        let out = propose(input).expect("matched");
        assert!(!out.contains("& mut"), "call-site mut not dropped: {out}");
        assert!(
            !out.contains("mut v :"),
            "binding mut not dropped: {out}"
        );
    }

    #[test]
    fn leaves_immutable_calls_unchanged() {
        let input = "fn count(v: &Vec<u8>) -> usize { v.len() }\nfn go(v: Vec<u8>) -> usize { count(&v) }";
        let out = propose(input);
        assert!(out.is_none(), "non-mut call touched: {out:?}");
    }
}

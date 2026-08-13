//! `clippy::unnecessary_unwrap` — `if x.is_some() { x.unwrap() } else { d }` →
//! `x.unwrap_or(d)`.

use syn::{Expr, ExprIf, File, Stmt};

pub(crate) fn rewrite(file: &mut File) -> bool {
    use syn::visit_mut::VisitMut;
    struct Visitor {
        changed: bool,
    }
    impl VisitMut for Visitor {
        fn visit_expr_mut(&mut self, e: &mut Expr) {
            syn::visit_mut::visit_expr_mut(self, e);
            if let Expr::If(if_expr) = e {
                if let Some(replacement) = try_unwrap_or(if_expr) {
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

fn try_unwrap_or(if_expr: &ExprIf) -> Option<Expr> {
    let receiver = match if_expr.cond.as_ref() {
        Expr::MethodCall(mc) if mc.method == "is_some" && mc.args.is_empty() => &*mc.receiver,
        _ => return None,
    };
    let then_stmts = &if_expr.then_branch.stmts;
    if then_stmts.len() != 1 {
        return None;
    }
    let _unwrap_call = match &then_stmts[0] {
        Stmt::Expr(Expr::MethodCall(mc), _) if mc.method == "unwrap" && mc.args.is_empty() => mc,
        _ => return None,
    };
    let else_expr = match &if_expr.else_branch {
        Some((_, else_e)) => else_e.as_ref(),
        None => return None,
    };
    let default = match else_expr {
        Expr::Block(b) => {
            if b.block.stmts.len() != 1 {
                return None;
            }
            match &b.block.stmts[0] {
                Stmt::Expr(e, _) => e,
                _ => return None,
            }
        }
        _ => return None,
    };
    let replacement: Expr = syn::parse_quote!(#receiver . unwrap_or ( #default ));
    Some(replacement)
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
    fn converts_if_unwrap_else_to_unwrap_or() {
        let input = "fn get(r: Option<u32>) -> u32 {\n    if r.is_some() { r.unwrap() } else { 7 }\n}";
        let out = propose(input).expect("matched");
        assert!(out.contains("unwrap_or"), "unwrap_or not produced: {out}");
    }

    #[test]
    fn leaves_plain_unwrap_unchanged() {
        let input = "fn get(r: Option<u32>) -> u32 { r.unwrap() }";
        let out = propose(input);
        assert!(out.is_none(), "plain unwrap touched: {out:?}");
    }

    #[test]
    fn leaves_if_without_else_unchanged() {
        let input = "fn get(r: Option<u32>) { if r.is_some() { r.unwrap() } }";
        let out = propose(input);
        assert!(out.is_none(), "no-else if touched: {out:?}");
    }
}

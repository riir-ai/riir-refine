//! `clippy::useless_conversion` — `let x: T = y.into()` → `let x: T = y`.

use syn::{Expr, File, ItemFn};

pub(crate) fn rewrite(file: &mut File) -> bool {
    let mut changed = false;
    for item in &mut file.items {
        if let syn::Item::Fn(fn_item) = item {
            changed |= rewrite_fn(fn_item);
        }
    }
    changed
}

fn rewrite_fn(fn_item: &mut ItemFn) -> bool {
    let mut changed = false;
    for stmt in &mut fn_item.block.stmts {
        if let syn::Stmt::Local(local) = stmt {
            // Must have an explicit type annotation + an init.
            let Some(init) = &mut local.init else {
                continue;
            };
            // Must also have a type annotation on the pattern.
            if !matches!(&local.pat, syn::Pat::Type(_)) {
                continue;
            }
            // init.expr must be a `.into()` method call.
            let new_expr = match init.expr.as_ref() {
                Expr::MethodCall(mc) if mc.method == "into" && mc.args.is_empty() => {
                    Some((*mc.receiver).clone())
                }
                _ => None,
            };
            if let Some(callee) = new_expr {
                *init.expr = callee;
                changed = true;
            }
        }
    }
    changed
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
    fn drops_redundant_into() {
        let input = "fn twice(n: u64) -> u64 {\n    let m: u64 = n.into();\n    m * 2\n}";
        let out = propose(input).expect("matched");
        assert!(!out.contains(".into"), "into not dropped: {out}");
        assert!(out.contains('n'), "callee lost: {out}");
    }

    #[test]
    fn leaves_non_into_let_unchanged() {
        let input = "fn twice(n: u64) -> u64 {\n    let m: u64 = n + 1;\n    m\n}";
        let out = propose(input);
        assert!(out.is_none(), "non-into let touched: {out:?}");
    }

    #[test]
    fn leaves_let_without_type_annotation() {
        let input = "fn twice(n: u64) -> u64 {\n    let m = n.into();\n    m\n}";
        let out = propose(input);
        assert!(out.is_none(), "no-type-annot let touched: {out:?}");
    }
}

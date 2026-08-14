//! `static-str` — `-> String { "lit".to_string() }` → `-> &'static str { "lit" }`.
//!
//! Pattern: a fn whose return type is `String` (bare or `std::string::String`)
//! and whose body is a single tail expression returning a string literal —
//! via `.to_string()`, `.to_owned()`, or `String::from(lit)`. The rewrite
//! drops the per-call heap allocation entirely (Bench 013: ParetoDominant
//! 36-43x).
//!
//! **Signature-visible change** (documented per the module caveat): callers
//! expecting `String` (e.g. `.len()` mutations, `format!("{s}") with owned
//! storage) see `&'static str`. Borrow-wise this is a widening (a
//! `&'static str` coerces anywhere a `&str` is expected), but owned-`String`
//! uses at call sites would need `.to_string()` — `rust_perf_verify`
//! (`cargo test`) is the semantic gate.

use syn::{Expr, ExprLit, File, Lit, ReturnType, Stmt, Type};

use super::for_each_fn_mut;

pub(crate) fn rewrite(file: &mut File) -> bool {
    let mut changed = false;
    for_each_fn_mut(file, |sig, block| {
        changed |= rewrite_fn(sig, block);
    });
    changed
}

fn rewrite_fn(sig: &mut syn::Signature, block: &mut syn::Block) -> bool {
    // Return type must be `String` (or `std::string::String`).
    let ReturnType::Type(_, ty) = &sig.output else { return false };
    if !is_string_ty(ty) {
        return false;
    }

    // Body must be exactly one tail expr returning a string literal.
    if block.stmts.len() != 1 {
        return false;
    }
    let Stmt::Expr(expr, None) = &block.stmts[0] else { return false };
    let Some(lit) = const_str_lit(expr) else { return false };

    sig.output = syn::parse_quote! { -> &'static str };
    block.stmts[0] = Stmt::Expr(
        Expr::Lit(ExprLit {
            attrs: Vec::new(),
            lit: Lit::Str(lit),
        }),
        None,
    );
    true
}

/// `String` or `std::string::String` (no qself, no generic args).
fn is_string_ty(ty: &Type) -> bool {
    let Type::Path(tp) = ty else { return false };
    if tp.qself.is_some() {
        return false;
    }
    let segs: Vec<String> = tp
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    segs == ["String"] || segs == ["std", "string", "String"]
}

/// Extract the `LitStr` from `"lit".to_string()` / `"lit".to_owned()` /
/// `String::from("lit")`.
fn const_str_lit(expr: &Expr) -> Option<syn::LitStr> {
    match expr {
        Expr::MethodCall(mc) if mc.args.is_empty() => {
            if mc.method != "to_string" && mc.method != "to_owned" {
                return None;
            }
            match &*mc.receiver {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) => Some(s.clone()),
                _ => None,
            }
        }
        Expr::Call(call) => {
            let Expr::Path(p) = &*call.func else { return None };
            if p.path.segments.len() != 2
                || p.path.segments[0].ident != "String"
                || p.path.segments[1].ident != "from"
            {
                return None;
            }
            match call.args.first() {
                Some(Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                })) => Some(s.clone()),
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::quote_file;
    use super::*;

    fn propose(input: &str) -> Option<String> {
        let mut file = syn::parse_file(input).ok()?;
        if rewrite(&mut file) {
            Some(quote_file(&file))
        } else {
            None
        }
    }

    fn squash(s: &str) -> String {
        s.split_whitespace().collect::<String>()
    }

    #[test]
    fn to_string_form_rewrites() {
        let out = propose("fn label() -> String { \"ready\".to_string() }").expect("pattern matched");
        assert!(squash(&out).contains(&squash("fn label() -> &'static str { \"ready\" }")), "{out}");
    }

    #[test]
    fn to_owned_form_rewrites() {
        let out = propose("fn name() -> String { \"katgpt\".to_owned() }").expect("pattern matched");
        assert!(squash(&out).contains(&squash("&'static str")), "{out}");
        assert!(squash(&out).contains("\"katgpt\""), "{out}");
    }

    #[test]
    fn string_from_form_rewrites() {
        let out = propose("fn tier() -> String { String::from(\"gold\") }").expect("pattern matched");
        assert!(squash(&out).contains(&squash("&'static str { \"gold\" }")), "{out}");
    }

    #[test]
    fn fully_qualified_return_type_rewrites() {
        let out = propose(
            "fn f() -> std::string::String { \"x\".to_string() }",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains(&squash("&'static str")), "{out}");
    }

    #[test]
    fn impl_method_rewrites() {
        let out = propose(
            "struct S;\nimpl S {\n    fn status(&self) -> String { \"ok\".to_string() }\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains(&squash("&'static str")), "{out}");
    }

    #[test]
    fn non_literal_expr_unchanged() {
        // Runtime-built strings stay.
        assert!(propose("fn f(x: u32) -> String { format!(\"n={x}\") }").is_none());
    }

    #[test]
    fn multi_stmt_body_unchanged() {
        assert!(
            propose("fn f() -> String { let s = \"a\".to_string(); s + \"b\" }").is_none()
        );
    }

    #[test]
    fn non_string_return_unchanged() {
        assert!(propose("fn f() -> u32 { 42 }").is_none());
        assert!(propose("fn f() -> Option<String> { None }").is_none());
    }
}

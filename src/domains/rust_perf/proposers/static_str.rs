//! `static-str` — `-> String { "lit".to_string() }` → `-> &'static str { "lit" }`.
//!
//! Pattern: a fn whose return type is `String` (bare or `std::string::String`)
//! and whose body is a single tail expression returning a string literal —
//! via `.to_string()`, `.to_owned()`, or `String::from(lit)`. The rewrite
//! drops the per-call heap allocation entirely (Bench 013: ParetoDominant
//! 36-43x).
//!
//! **Signature-visible change** (documented per the module caveat): callers
//! expecting owned `String` would need `.to_string()` — `rust_perf_verify`
//! (`cargo test`) is the semantic gate.
//!
//! Span-preserving: two edits per matched fn — the return type span →
//! `-> &'static str`, and the tail expr span → the literal's own original
//! text. Non-overlapping by construction (signature vs body).

use syn::{Expr, ExprLit, File, Lit, ReturnType, Stmt, Type};

use super::{edit, src, SourceEdit};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    super::for_each_fn(file, |sig, block| {
        fn_edits(source, sig, block, &mut edits);
    });
    edits
}

/// Push the fn's edits (0 or 2) if it matches.
fn fn_edits(source: &str, sig: &syn::Signature, block: &syn::Block, edits: &mut Vec<SourceEdit>) {
    // Return type must be `String` (or `std::string::String`).
    let ReturnType::Type(_, ty) = &sig.output else { return };
    if !is_string_ty(ty) {
        return;
    }

    // Body must be exactly one tail expr returning a string literal.
    if block.stmts.len() != 1 {
        return;
    }
    let Stmt::Expr(expr, None) = &block.stmts[0] else { return };
    let Some(lit) = const_str_lit(expr) else { return };

    // Tail edit: the literal's OWN source text replaces the whole
    // `.to_string()`-wrapping expression span.
    let Some(tail_text) = src(source, &lit) else { return };
    edits.push(edit(&sig.output, "-> &'static str".to_string()));
    edits.push(edit(expr, tail_text));
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
    use super::super::propose_fix;

    fn squash(s: &str) -> String {
        s.split_whitespace().collect::<String>()
    }

    #[test]
    fn to_string_form_rewrites() {
        let out = propose_fix("static-str", "fn label() -> String { \"ready\".to_string() }")
            .expect("pattern matched");
        assert!(squash(&out).contains(&squash("fn label() -> &'static str { \"ready\" }")), "{out}");
    }

    #[test]
    fn to_owned_form_rewrites() {
        let out = propose_fix("static-str", "fn name() -> String { \"katgpt\".to_owned() }")
            .expect("pattern matched");
        assert!(squash(&out).contains(&squash("&'static str")), "{out}");
        assert!(out.contains("\"katgpt\""), "{out}");
    }

    #[test]
    fn string_from_form_rewrites() {
        let out = propose_fix("static-str", "fn tier() -> String { String::from(\"gold\") }")
            .expect("pattern matched");
        assert!(squash(&out).contains(&squash("&'static str { \"gold\" }")), "{out}");
    }

    #[test]
    fn fully_qualified_return_type_rewrites() {
        let out = propose_fix("static-str", "fn f() -> std::string::String { \"x\".to_string() }")
            .expect("pattern matched");
        assert!(squash(&out).contains(&squash("&'static str")), "{out}");
    }

    #[test]
    fn impl_method_rewrites() {
        let out = propose_fix(
            "static-str",
            "struct S;\nimpl S {\n    fn status(&self) -> String { \"ok\".to_string() }\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains(&squash("&'static str")), "{out}");
    }

    #[test]
    fn multi_line_formatting_preserved() {
        let out = propose_fix(
            "static-str",
            "fn label()\n    -> String\n{\n    \"ready\"\n        .to_string()\n}",
        )
        .expect("pattern matched");
        // Only the two matched spans changed; the line structure survives.
        assert!(out.contains("fn label()"), "{out}");
        assert!(out.contains("-> &'static str"), "{out}");
        assert!(out.contains("\"ready\""), "{out}");
        assert!(squash(&out).contains(&squash("-> &'static str { \"ready\" }")), "{out}");
    }

    #[test]
    fn non_literal_expr_unchanged() {
        // Runtime-built strings stay.
        assert!(propose_fix("static-str", "fn f(x: u32) -> String { format!(\"n={x}\") }").is_none());
    }

    #[test]
    fn multi_stmt_body_unchanged() {
        assert!(
            propose_fix("static-str", "fn f() -> String { let s = \"a\".to_string(); s + \"b\" }")
                .is_none()
        );
    }

    #[test]
    fn non_string_return_unchanged() {
        assert!(propose_fix("static-str", "fn f() -> u32 { 42 }").is_none());
        assert!(propose_fix("static-str", "fn f() -> Option<String> { None }").is_none());
    }
}

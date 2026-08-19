//! `static-str` — `-> String { "lit".to_string() }` → `-> &'static str { "lit" }`.
//!
//! Pattern: a fn whose return type is `String` (bare or `std::string::String`)
//! and whose body is a single tail expression returning a string literal —
//! via `.to_string()`, `.to_owned()`, or `String::from(lit)`. The rewrite
//! drops the per-call heap allocation entirely (Bench 013: ParetoDominant
//! 36-43x).
//!
//! **`const` emission (2026-08-19)**: the rewritten shape (free/inherent-impl
//! fn, literal tail) is trivially const-able, and real clippy's
//! `missing_const_for_fn` (pedantic) fires on the non-const result — the
//! run-#2 bench caught exactly this as 2 of the 3 created lints. The
//! rewrite now emits `const fn` alongside (additive — a const fn is
//! callable everywhere a fn was, including fn-pointer coercion). Skipped
//! for `async`/`unsafe` fns (qualifier-order/grammar risk for zero benefit)
//! and trait impls (below).
//!
//! **Trait-impl soundness fix (2026-08-19)**: a trait-impl fn's signature
//! MUST match the trait's declaration — rewriting its return type is always
//! a compile error. The old walker (`for_each_fn`) could not distinguish
//! trait impls from inherent impls (`ImplItem::Fn` covers both) and fired
//! on both. This proposer now walks with trait-awareness and skips trait
//! impls entirely.
//!
//! **Signature-visible change** (documented per the module caveat): callers
//! expecting owned `String` would need `.to_string()` — `rust_perf_verify`
//! (`cargo test`) is the semantic gate.
//!
//! Span-preserving: two or three edits per matched fn — the return type span
//! → `-> &'static str`, the tail expr span → the literal's own original
//! text, and (when const is emitted) a zero-width insertion of `const `
//! before the `fn` keyword. Non-overlapping by construction.

use syn::spanned::Spanned;
use syn::{Expr, ExprLit, File, Item, Lit, ReturnType, Stmt, Type};

use super::{edit, src, SourceEdit};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    walk_items(&file.items, source, &mut edits);
    edits
}

/// Trait-aware fn walk: free fns + INHERENT impl fns + inline mods +
/// block-nested fns. Trait impls are skipped entirely (a signature rewrite
/// there can never compile). Under-approximates `for_each_fn` only on
/// expr-nested fns (if/match/loop bodies) — no observed production shape.
fn walk_items(items: &[Item], source: &str, edits: &mut Vec<SourceEdit>) {
    for item in items {
        match item {
            Item::Fn(f) => {
                fn_edits(source, &f.sig, &f.block, edits);
                walk_block(&f.block, source, edits);
            }
            Item::Impl(imp) => {
                if imp.trait_.is_some() {
                    continue; // trait-impl signatures must match the trait
                }
                for ii in &imp.items {
                    if let syn::ImplItem::Fn(m) = ii {
                        fn_edits(source, &m.sig, &m.block, edits);
                        walk_block(&m.block, source, edits);
                    }
                }
            }
            Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    walk_items(inner, source, edits);
                }
            }
            _ => {}
        }
    }
}

/// Fns declared as statements inside a fn body (any block depth).
fn walk_block(block: &syn::Block, source: &str, edits: &mut Vec<SourceEdit>) {
    for stmt in &block.stmts {
        if let Stmt::Item(Item::Fn(f)) = stmt {
            fn_edits(source, &f.sig, &f.block, edits);
            walk_block(&f.block, source, edits);
        }
    }
}

/// Push the fn's edits (0, 2, or 3) if it matches.
fn fn_edits(source: &str, sig: &syn::Signature, block: &syn::Block, edits: &mut Vec<SourceEdit>) {
    // Caller gate (the action_hoare.rs E0308 lesson, applied to the return-
    // type ripple): any other reference to this fn in the file is a caller
    // whose `let s: String = label()` binding a span-local edit cannot adapt.
    if super::count_ident_occurrences(source, &sig.ident.to_string()) > 1 {
        return;
    }
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

    // const emission: additive, and it closes the `missing_const_for_fn`
    // pedantic lint clippy fires on the non-const rewrite (probe-verified
    // 2026-08-19: `pub const fn f() -> &'static str { "lit" }` is clean).
    // Skipped when the grammar gets interesting (async/unsafe qualifiers)
    // or the fn is already const.
    if sig.constness.is_none()
        && sig.asyncness.is_none()
        && sig.unsafety.is_none()
    {
        let fn_start = sig.fn_token.span().byte_range().start;
        edits.push(SourceEdit {
            start: fn_start,
            end: fn_start,
            replacement: "const ".to_string(),
        });
    }
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
        assert!(squash(&out).contains(&squash("const fn label() -> &'static str { \"ready\" }")), "{out}");
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
    fn inherent_impl_method_rewrites_with_const() {
        let out = propose_fix(
            "static-str",
            "struct S;\nimpl S {\n    fn status(&self) -> String { \"ok\".to_string() }\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains(&squash("const fn status(&self) -> &'static str")), "{out}");
    }

    #[test]
    fn trait_impl_fn_untouched() {
        // A trait-impl fn's signature MUST match the trait — rewriting the
        // return type there is always a compile error. The old walker fired
        // on trait impls (`ImplItem::Fn` covers both impl kinds).
        let input = "trait T { fn label() -> String; }\nstruct S;\nimpl T for S {\n    fn label() -> String { \"x\".to_string() }\n}";
        let out = propose_fix("static-str", input);
        assert!(out.is_none(), "trait-impl fn must not fire: {out:?}");
    }

    #[test]
    fn async_fn_skips_const() {
        let out = propose_fix(
            "static-str",
            "async fn label() -> String { \"ready\".to_string() }",
        )
        .expect("pattern matched");
        assert!(out.contains("async fn label()"), "async must stay: {out}");
        assert!(!out.contains("const"), "const skipped for async: {out}");
        assert!(out.contains("&'static str"), "{out}");
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
    fn live_caller_blocks_rewrite() {
        // The return-type ripple: a caller binding the result as owned
        // `String` breaks when the signature becomes `&'static str` — a
        // span-local edit cannot adapt the caller. Refuse.
        assert!(
            propose_fix(
                "static-str",
                "fn label() -> String { \"ready\".to_string() }\nfn user() -> String { label() }",
            )
            .is_none()
        );
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

//! Shared proposer helpers (Issue 016 structural-lint expansion).
//!
//! Small predicate/shape helpers used by multiple per-lint proposers.
//! Kept here (not per-file) because the families overlap: the postfix-safe
//! check is needed by every proposer that reconstructs `<expr>.method(..)`,
//! the logic-paren check by every one that negates or `&&`-joins conditions,
//! and the Option/Result pattern predicates by the match-family proposers
//! (the `manual_map` precedent kept these private — with 30 lints the
//! duplication stopped being trivial).

use proc_macro2::TokenStream;
use syn::spanned::Spanned;
use syn::{Expr, ExprLit, Lit, Macro, Pat};

/// Can `<expr>.method(...)` be appended without changing grouping? Postfix
/// chains (paths, calls, field/index access, `?`, awaits, literals, macros)
/// bind tighter than `.`; binary/unary/if/struct forms need parens. Casts
/// are EXCLUDED despite parsing fine in operator position: method calls bind
/// TIGHTER than `as`, so `x as u64.method()` groups the method into the
/// TYPE position and fails — a cast receiver must parenthesize
/// (`(x as u64).method()`; P11, found authoring `manual_is_multiple_of`).
pub(crate) fn is_postfix_safe(e: &Expr) -> bool {
    matches!(
        e,
        Expr::Path(_)
            | Expr::Call(_)
            | Expr::MethodCall(_)
            | Expr::Field(_)
            | Expr::Index(_)
            | Expr::Paren(_)
            | Expr::Try(_)
            | Expr::Await(_)
            | Expr::Lit(_)
            | Expr::Macro(_)
    )
}

/// Does `<expr>` need parens when negated (`!e`), `&&`-joined, or used as a
/// replacement in operator position? The sets DIVERGE deliberately at casts:
/// `x as u64 == y` parses fine (`as` binds tighter than binary operators)
/// but `x as u64.method()` does not — casts pass here and fail
/// [`is_postfix_safe`]. Over-parenthesizing a cast condition is byte-drift
/// vs the oracle, harmless; under-parenthesizing before `.` is a break.
pub(crate) fn needs_logic_parens(e: &Expr) -> bool {
    !is_postfix_safe(e)
}

/// Does the expression contain a `let` condition anywhere (directly or in
/// `&&`/paren operands)? Rewrites that move such a condition into `&&`-join
/// or negation position are unrepresentable: `x && (let a = b)` and
/// `!(let a = b)` do not parse (a parenthesized `let` is never valid Rust),
/// and the un-parenthesized let-chain form is edition-2024-only — a
/// parse-only proposer cannot see the target crate's edition. Proposers
/// refuse instead (the `map_unwrap_or` array-literal guard class: our output
/// must compile on every edition; real clippy gates the let-chain fold on
/// edition >= 2024).
pub(crate) fn contains_let(e: &Expr) -> bool {
    struct Seeker {
        found: bool,
    }
    impl<'ast> syn::visit::Visit<'ast> for Seeker {
        fn visit_expr_let(&mut self, _i: &'ast syn::ExprLet) {
            self.found = true;
        }
    }
    let mut s = Seeker { found: false };
    syn::visit::Visit::visit_expr(&mut s, e);
    s.found
}

/// Side-effect-free expression whitelist — for rewrites that move an
/// expression from a lazily-evaluated position (match arm, closure body)
/// into an eagerly-evaluated one (argument position). Paths, literals,
/// field access, and paren/binary/unary compositions of those are safe;
/// calls, method calls, macros, indexing (can panic), `?`, and awaits are
/// not.
pub(crate) fn is_pure_expr(e: &Expr) -> bool {
    match e {
        Expr::Path(_) | Expr::Lit(_) => true,
        Expr::Paren(p) => is_pure_expr(&p.expr),
        Expr::Unary(u) => is_pure_expr(&u.expr),
        Expr::Binary(b) => is_pure_expr(&b.left) && is_pure_expr(&b.right),
        // Struct literal — construction has no effects; pure iff every
        // field value (and the struct-update base, if any) is pure.
        // Measured against real clippy 2026-09-09 (probe crate,
        // unnecessary_lazy_evaluations): flagged with pure fields, NOT
        // flagged when any field calls a fn.
        Expr::Struct(s) => {
            s.fields.iter().all(|f| is_pure_expr(&f.expr))
                && s.rest.as_ref().is_none_or(|r| is_pure_expr(r))
        }
        // Field access — reads have no effects; pure iff the base is.
        Expr::Field(f) => is_pure_expr(&f.base),
        // The std enum constructors can't have side effects — anything
        // else callable might. Bounded: `Some`/`Ok`/`Err` ONLY, args pure
        // (real clippy flags `|| Some(1)` / `|| Err(e)`, never an
        // arbitrary call).
        Expr::Call(c) => {
            let Expr::Path(p) = &*c.func else {
                return false;
            };
            let ctor = p
                .path
                .segments
                .last()
                .map_or_else(String::new, |s| s.ident.to_string());
            matches!(ctor.as_str(), "Some" | "Ok" | "Err") && c.args.iter().all(is_pure_expr)
        }
        // Indexing can panic (real clippy does NOT flag `|| a[0]`),
        // method calls / macros / arrays can do anything — stay impure.
        _ => false,
    }
}

/// Integer literal accessor: `(digits, suffix)` of an `Expr::Lit` holding a
/// base-10 integer literal, `None` otherwise. The suffix is returned
/// WITHOUT the `_` separator (e.g. `0u32` → `("0", "u32")`).
pub(crate) fn int_lit(e: &Expr) -> Option<(String, String)> {
    let Expr::Lit(ExprLit {
        lit: Lit::Int(i), ..
    }) = e
    else {
        return None;
    };
    let digits = i.base10_digits().to_string();
    let suffix = i.suffix().trim_start_matches('_').to_string();
    Some((digits, suffix))
}

/// Boolean literal accessor for `true`/`false` expressions.
pub(crate) fn bool_lit(e: &Expr) -> Option<bool> {
    let Expr::Lit(ExprLit {
        lit: Lit::Bool(b), ..
    }) = e
    else {
        return None;
    };
    Some(b.value)
}

/// Is this arm body the unit expression (`()` or an empty block `{}`)?
pub(crate) fn is_unit_expr(e: &Expr) -> bool {
    match e {
        Expr::Tuple(t) => t.elems.is_empty(),
        Expr::Block(b) => b.block.stmts.is_empty(),
        _ => false,
    }
}

/// Is this pattern the bare `None` constructor? (`None` can parse as
/// `Pat::Path`, `Pat::TupleStruct`(empty), or `Pat::Ident` — syn ambiguity;
/// check the token text, same as `manual_map`.)
pub(crate) fn is_none_pat(p: &Pat) -> bool {
    matches!(p, Pat::Path(pp) if pp.path.is_ident("None"))
        || matches!(p, Pat::TupleStruct(ts) if ts.path.is_ident("None") && ts.elems.is_empty())
        || matches!(p, Pat::Ident(pi) if pi.ident == "None")
}

/// Comment-displacement guard — hoisted to the feature-neutral emitter
/// substrate (Plan 057 salvage: the `sec` NaN-comparator proposer shares it
/// without pulling `clippy_lints` into `--no-default-features --features
/// sec`).
pub(crate) use crate::edit_emitter::{comment_marker_positions, drops_interior_comment};

/// Parse a format-family macro's post-template arguments into `Expr`s (the
/// `map_unwrap_or` macro lane, extracted 2026-09-14 when `unnecessary_map_or`
/// gained the same descent). Non-format macros return an empty vec; named /
/// non-expression args are skipped (a `println!("{a}", a = 1, ..)` named arg
/// never aborts scanning of later args).
pub(crate) fn macro_expr_args(mac: &Macro) -> Vec<Expr> {
    let Some(args) = super::uninlined_format_args::post_template_expr_args(mac) else {
        return Vec::new();
    };
    args.into_iter()
        .filter_map(|a| {
            let stream: TokenStream = a.toks.iter().cloned().collect();
            syn::parse2::<Expr>(stream).ok()
        })
        .collect()
}

/// Span helper: byte range of any spanned node (kept-fragment input).
pub(crate) fn byte_range<T: Spanned>(t: &T) -> std::ops::Range<usize> {
    t.span().byte_range()
}

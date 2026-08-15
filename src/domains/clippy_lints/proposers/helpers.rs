//! Shared proposer helpers (Issue 016 structural-lint expansion).
//!
//! Small predicate/shape helpers used by multiple per-lint proposers.
//! Kept here (not per-file) because the families overlap: the postfix-safe
//! check is needed by every proposer that reconstructs `<expr>.method(..)`,
//! the logic-paren check by every one that negates or `&&`-joins conditions,
//! and the Option/Result pattern predicates by the match-family proposers
//! (the `manual_map` precedent kept these private — with 30 lints the
//! duplication stopped being trivial).

use syn::{Expr, ExprLit, Lit, Pat};

/// Can `<expr>.method(...)` be appended without changing grouping? Postfix
/// chains (paths, calls, field/index access, `?`, awaits, literals, macros)
/// bind tighter than `.`; binary/unary/if/struct forms need parens.
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
            | Expr::Cast(_)
    )
}

/// Does `<expr>` need parens when negated (`!e`), `&&`-joined, or used as a
/// replacement in operator position? Complementary to [`is_postfix_safe`]
/// modulo the forms that parse fine in logic position but not before `.`
/// (none today — the sets coincide; kept separate for intent).
pub(crate) fn needs_logic_parens(e: &Expr) -> bool {
    !is_postfix_safe(e)
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
        _ => false,
    }
}

/// Integer literal accessor: `(digits, suffix)` of an `Expr::Lit` holding a
/// base-10 integer literal, `None` otherwise. The suffix is returned
/// WITHOUT the `_` separator (e.g. `0u32` → `("0", "u32")`).
pub(crate) fn int_lit(e: &Expr) -> Option<(String, String)> {
    let Expr::Lit(ExprLit { lit: Lit::Int(i), .. }) = e else {
        return None;
    };
    let digits = i.base10_digits().to_string();
    let suffix = i.suffix().trim_start_matches('_').to_string();
    Some((digits, suffix))
}

/// Boolean literal accessor for `true`/`false` expressions.
pub(crate) fn bool_lit(e: &Expr) -> Option<bool> {
    let Expr::Lit(ExprLit { lit: Lit::Bool(b), .. }) = e else {
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

//! Shared proposer helpers (Issue 016 structural-lint expansion).
//!
//! Small predicate/shape helpers used by multiple per-lint proposers.
//! Kept here (not per-file) because the families overlap: the postfix-safe
//! check is needed by every proposer that reconstructs `<expr>.method(..)`,
//! the logic-paren check by every one that negates or `&&`-joins conditions,
//! and the Option/Result pattern predicates by the match-family proposers
//! (the `manual_map` precedent kept these private — with 30 lints the
//! duplication stopped being trivial).

use syn::spanned::Spanned;
use syn::{Expr, ExprLit, Lit, Pat};

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

/// Does a whole-node splice replacement DROP a comment? Scans the replaced
/// span's source for comment markers and reports any that is NOT inside a
/// kept fragment (Issue 027/028/029 class — real clippy stays silent on
/// comment-displacing rewrites, and our emitter deletes the comment
/// outright). Kept fragments are byte ranges that survive verbatim in the
/// replacement (e.g. scrutinee, patterns, arm bodies).
pub(crate) fn drops_interior_comment(
    source: &str,
    span: std::ops::Range<usize>,
    kept: &[std::ops::Range<usize>],
) -> bool {
    let Some(text) = source.get(span.clone()) else {
        return true; // broken span bookkeeping — refuse rather than displace
    };
    let offset = span.start;
    for pos in comment_marker_positions(text) {
        let abs = offset + pos;
        if !kept.iter().any(|k| k.contains(&abs)) {
            return true;
        }
    }
    false
}

/// Byte offsets of `//` and `/*` comment OPENERS in `text` (syn has no
/// comment nodes). Dropped-gap regions of a rewritten node contain only
/// keywords/punctuation, so string-literal false positives cannot occur
/// there; kept fragments are checked verbatim so a `//` inside a string in a
/// kept fragment is covered by its own range.
pub(crate) fn comment_marker_positions(text: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && (bytes[i + 1] == b'/' || bytes[i + 1] == b'*') {
            out.push(i);
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}

/// Span helper: byte range of any spanned node (kept-fragment input).
pub(crate) fn byte_range<T: Spanned>(t: &T) -> std::ops::Range<usize> {
    t.span().byte_range()
}

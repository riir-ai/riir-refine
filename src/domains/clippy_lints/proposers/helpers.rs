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
use syn::{Attribute, Expr, ExprLit, File, Lit, Macro, Pat};

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

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn count_in(source: &str, ident: &str, skip_dot_prefixed: bool) -> usize {
    let bytes = source.as_bytes();
    let mut count = 0;
    let mut from = 0;
    while let Some(rel) = source[from..].find(ident) {
        let at = from + rel;
        let end = at + ident.len();
        let before_ok = at == 0 || !is_ident_byte(bytes[at - 1]);
        let after_ok = end >= bytes.len() || !is_ident_byte(bytes[end]);
        let not_field_or_method = at == 0 || bytes[at - 1] != b'.';
        if before_ok && after_ok && (!skip_dot_prefixed || not_field_or_method) {
            count += 1;
        }
        from = end;
    }
    count
}

/// Word-boundary ident occurrence count EXCLUDING method/field positions
/// (occurrences immediately preceded by `.`) — for binding-use analysis.
/// `v.iter().sum()` contributes NOTHING to `sum`'s count: the method
/// namespace is not the binding namespace (Batch 155, `let_and_return`).
pub(crate) fn count_binding_occurrences(source: &str, ident: &str) -> usize {
    count_in(source, ident, true)
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

/// One comment-style doc line, resolved for the doc-lint proposers
/// (`doc_markdown` + the Batch 151 doc-line state machine — the shared walk
/// both consume; extracted 2026-09-17 when the second consumer landed).
pub(crate) struct DocLine<'a> {
    /// Byte offset of the doc TEXT (right after the `///`/`//!` marker).
    pub base: usize,
    /// The doc text (everything after the marker, verbatim — including the
    /// conventional leading space — no trailing newline).
    pub text: &'a str,
    /// True when a source-line gap precedes this attr (a new doc block:
    /// cross-line state must not leak from the previous block).
    pub new_block: bool,
}

/// Walk every comment-style doc line in source order. Doc comments are
/// located through REAL `#[doc]` attributes (comment-style only — the source
/// at the attribute's byte range must start `///` or `//!`); a raw line scan
/// cannot distinguish a doc comment from a STRING LITERAL containing `///`
/// text. `#[doc = "..."]` attribute-form docs are skipped by construction
/// (the +3 marker slicing only holds for comment-style). Macro token trees
/// are opaque to syn, so doc-shaped text inside `macro_rules!` bodies is
/// invisible (silent divergence, the safe direction).
pub(crate) fn doc_lines<'a>(source: &'a str, file: &File) -> Vec<DocLine<'a>> {
    use syn::visit::Visit as _;

    struct Collector {
        docs: Vec<std::ops::Range<usize>>,
    }
    impl<'ast> syn::visit::Visit<'ast> for Collector {
        fn visit_attribute(&mut self, attr: &'ast Attribute) {
            if attr.path().is_ident("doc") {
                self.docs.push(byte_range(attr));
            }
            syn::visit::visit_attribute(self, attr);
        }
    }

    let mut v = Collector { docs: Vec::new() };
    v.visit_file(file);
    // Visit order is roughly source order but not guaranteed across item
    // kinds — sort, then walk in line order.
    v.docs.sort_by_key(|r| r.start);
    let mut out = Vec::with_capacity(v.docs.len());
    let mut prev_line: Option<usize> = None;
    let mut line_no = 0usize;
    let mut scanned = 0usize;
    for range in v.docs {
        // Line number of this doc attr (for the adjacency gap flag).
        while scanned < range.start {
            if source.as_bytes()[scanned] == b'\n' {
                line_no += 1;
            }
            scanned += 1;
        }
        let new_block = prev_line.is_some_and(|p| line_no.saturating_sub(p) > 1);
        prev_line = Some(line_no);
        let line_end = source[range.start..]
            .find('\n')
            .map_or(source.len(), |i| range.start + i);
        let line = &source[range.start..line_end];
        // Comment-style only: the marker is exactly 3 chars, and the doc
        // text (syn's `#[doc]` value) is the rest of the line verbatim.
        let Some(_marker) = line.strip_prefix("///").or_else(|| line.strip_prefix("//!")) else {
            continue;
        };
        out.push(DocLine {
            base: range.start + 3,
            text: &source[range.start + 3..line_end],
            new_block,
        });
    }
    out.shrink_to_fit();
    out
}

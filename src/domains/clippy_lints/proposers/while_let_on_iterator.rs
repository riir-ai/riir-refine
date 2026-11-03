//! `clippy::while_let_on_iterator` — `while let Some(x) = it.next() { .. }` →
//! `for x in it { .. }`.
//!
//! VERIFY-tier: the receiver could be a custom non-Iterator type with a
//! `next()` method — rustc backs the revert under `--verify`.
//!
//! Detection is deliberately conservative: the iterator must be a BARE
//! ident (`it.next()`), the binding must be a real ident (not `_`), and the
//! loop BODY must not mention the iterator ident again (mutation/reuse —
//! the `for` rewrite would move it). Binding modifiers (`mut`, `ref`) are
//! preserved by splicing the binding's ORIGINAL source text.
//! Span-preserving (T6.4): the whole `while` expr's byte range is replaced
//! by `for <binding> in <IT> <body>` (body = the block's own source text).

use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Block, Expr, ExprLet, ExprMethodCall, ExprWhile, File, Ident, Pat, PatIdent};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_while(&mut self, w: &'ast ExprWhile) {
        syn::visit::visit_expr_while(self, w);
        if let Some(replacement) = self.try_for_loop(w) {
            self.edits.push(edit(w, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_for_loop(&self, w: &ExprWhile) -> Option<String> {
        if !w.attrs.is_empty() || w.label.is_some() {
            return None; // attrs/label would be dropped by the splice
        }
        let Expr::Let(le) = &*w.cond else {
            return None;
        };
        let (binding, next) = some_binding_over_next(le)?;
        // Iterator must be a bare ident (`it`), not a chain (`xs.iter()`).
        let Expr::Path(it_path) = &*next.receiver else {
            return None;
        };
        let it_ident = it_path.path.get_ident()?;
        // Guard: the body must not mention the iterator ident (mutation,
        // reuse, or a same-named binding would break under `for`).
        if body_mentions(&w.body, it_ident) {
            return None;
        }
        let binding = src(self.source, binding)?;
        let iter = src(self.source, &next.receiver)?;
        let body = src(self.source, &w.body)?;
        Some(format!("for {binding} in {iter} {body}"))
    }
}

/// Match `let Some(<binding>) = <recv>.next()` — returns the binding pattern
/// and the (already validated) `next` method call.
fn some_binding_over_next(le: &ExprLet) -> Option<(&PatIdent, &ExprMethodCall)> {
    if !le.attrs.is_empty() {
        return None;
    }
    let Pat::TupleStruct(ts) = &*le.pat else {
        return None;
    };
    if !ts.attrs.is_empty() || !ts.path.is_ident("Some") || ts.elems.len() != 1 {
        return None;
    }
    let Pat::Ident(binding) = &ts.elems[0] else {
        return None;
    };
    if !binding.attrs.is_empty() || binding.subpat.is_some() || binding.ident == "_" {
        return None;
    }
    let Expr::MethodCall(next) = &*le.expr else {
        return None;
    };
    if !next.attrs.is_empty()
        || next.turbofish.is_some()
        || next.method != "next"
        || !next.args.is_empty()
    {
        return None;
    }
    Some((binding, next))
}

/// Does any path in the block mention `name`?
fn body_mentions(body: &Block, name: &Ident) -> bool {
    let mut scan = PathMention { name, found: false };
    scan.visit_block(body);
    scan.found
}

struct PathMention<'a> {
    name: &'a Ident,
    found: bool,
}

impl<'ast> Visit<'ast> for PathMention<'_> {
    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if p.path.get_ident() == Some(self.name) {
            self.found = true;
        }
        syn::visit::visit_expr_path(self, p);
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_while_let_to_for() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) -> u32 { let mut s = 0; while let Some(x) = it.next() { s += x as u32; } s }";
        let out = propose_fix("while_let_on_iterator", input).expect("matched");
        assert!(
            out.contains("for x in it { s += x as u32; }"),
            "for not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn mut_binding_modifier_preserved() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) { while let Some(mut x) = it.next() { x += 1; } }";
        let out = propose_fix("while_let_on_iterator", input).expect("matched");
        assert!(
            out.contains("for mut x in it"),
            "binding modifier lost: {out}"
        );
    }

    #[test]
    fn body_mentioning_iterator_unchanged() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) { while let Some(x) = it.next() { it.reset(); } }";
        let out = propose_fix("while_let_on_iterator", input);
        assert!(
            out.is_none(),
            "body reuses the iterator — must not fire: {out:?}"
        );
    }

    #[test]
    fn wildcard_binding_unchanged() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) { while let Some(_) = it.next() {} }";
        let out = propose_fix("while_let_on_iterator", input);
        assert!(out.is_none(), "Some(_) must not fire: {out:?}");
    }

    #[test]
    fn method_chain_receiver_unchanged() {
        let input = "fn f(xs: &Vec<u8>) { while let Some(x) = xs.iter().next() {} }";
        let out = propose_fix("while_let_on_iterator", input);
        assert!(out.is_none(), "non-ident receiver must not fire: {out:?}");
    }
}

//! `clippy::iter_next_loop` — `for x in it.next() { }` → `for x in it { }`.
//!
//! Correctness lint: iterating an `Option` runs the body 0-or-1 times — the
//! author almost certainly meant to iterate the iterator itself. The fix
//! removes the `.next()`. Span-preserving (T6.4): ONLY the `iter` expr's byte
//! range (`it.next()`) is replaced by the ORIGINAL source text of the
//! method-call's receiver — the `for` keyword, pattern, and body block stay
//! as untouched bytes.

use super::helpers::is_postfix_safe;
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprForLoop, File};

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
    fn visit_expr_for_loop(&mut self, f: &'ast ExprForLoop) {
        syn::visit::visit_expr_for_loop(self, f);
        if let Some(replacement) = self.try_drop_next(f) {
            self.edits.push(edit(&f.expr, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_drop_next(&self, f: &ExprForLoop) -> Option<String> {
        let Expr::MethodCall(next) = &*f.expr else {
            return None; // `for x in <non-method>` — nothing to drop
        };
        if !next.attrs.is_empty() || next.turbofish.is_some() || next.method != "next" {
            return None;
        }
        if !next.args.is_empty() {
            return None;
        }
        let recv = src(self.source, &next.receiver)?;
        Some(if is_postfix_safe(&next.receiver) {
            recv
        } else {
            format!("({recv})")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn drops_next_from_for_iter() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0u8; for x in it.next() { s += x; } s }";
        let out = propose_fix("iter_next_loop", input).expect("matched");
        assert!(
            out.contains("for x in it { s += x; }"),
            "next not dropped: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn chain_receiver_preserved() {
        let input = "fn g(v: &Vec<u8>) { for x in v.iter().next() {} }";
        let out = propose_fix("iter_next_loop", input).expect("matched");
        assert!(
            out.contains("for x in v.iter() {}"),
            "receiver text not preserved: {out}"
        );
    }

    #[test]
    fn plain_for_unchanged() {
        let input = "fn h(mut it: std::vec::IntoIter<u8>) { for x in it {} }";
        let out = propose_fix("iter_next_loop", input);
        assert!(out.is_none(), "plain for-loop must not fire: {out:?}");
    }

    #[test]
    fn non_next_method_unchanged() {
        let input = "fn h(v: &Vec<u8>) { for x in v.iter().by_ref() {} }";
        let out = propose_fix("iter_next_loop", input);
        assert!(out.is_none(), "only .next() receivers fire: {out:?}");
    }
}

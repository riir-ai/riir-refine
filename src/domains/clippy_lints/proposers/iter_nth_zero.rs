//! `clippy::iter_nth_zero` — `xs.iter().nth(0)` → `xs.iter().next()`.
//!
//! `Iterator::nth(0)` is `next()` with an extra argument (and, on some
//! adapters, extra bookkeeping). Any integer-suffix spelling of zero fires
//! (`0`, `0usize`). Span-preserving (T6.4): the whole `nth` call's byte range
//! is replaced by `<recv>.next()`, the receiver being the ORIGINAL source
//! text of the matched node.

use super::helpers::{int_lit, is_postfix_safe};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{ExprMethodCall, File};

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
    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        if let Some(replacement) = self.try_next(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_next(&self, m: &ExprMethodCall) -> Option<String> {
        if !m.attrs.is_empty() || m.turbofish.is_some() || m.method != "nth" {
            return None;
        }
        if m.args.len() != 1 {
            return None;
        }
        if int_lit(&m.args[0]).is_none_or(|(digits, _)| digits != "0") {
            return None; // nth(n != 0) genuinely skips
        }
        let recv = src(self.source, &m.receiver)?;
        let recv = if is_postfix_safe(&m.receiver) {
            recv
        } else {
            format!("({recv})")
        };
        Some(format!("{recv}.next()"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_nth_zero_to_next() {
        let input = "fn f(xs: &Vec<u8>) -> Option<&u8> { xs.iter().nth(0) }";
        let out = propose_fix("iter_nth_zero", input).expect("matched");
        assert!(out.contains("xs.iter().next()"), "next not produced: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn any_zero_suffix_fires() {
        let input = "fn f(xs: &Vec<u8>) -> Option<&u8> { xs.iter().nth(0usize) }";
        let out = propose_fix("iter_nth_zero", input).expect("matched");
        assert!(
            out.contains("xs.iter().next()"),
            "suffixed zero must fire: {out}"
        );
    }

    #[test]
    fn nth_nonzero_unchanged() {
        let input = "fn f(xs: &Vec<u8>) -> Option<&u8> { xs.iter().nth(1) }";
        let out = propose_fix("iter_nth_zero", input);
        assert!(
            out.is_none(),
            "nth(1) genuinely skips — must not fire: {out:?}"
        );
    }

    #[test]
    fn bare_next_unchanged() {
        let input = "fn f(xs: &Vec<u8>) -> Option<&u8> { xs.iter().next() }";
        let out = propose_fix("iter_nth_zero", input);
        assert!(out.is_none(), "already next() must not fire: {out:?}");
    }
}

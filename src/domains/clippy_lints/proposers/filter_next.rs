//! `clippy::filter_next` — `xs.iter().filter(p).next()` → `xs.iter().find(p)`.
//!
//! `Iterator::find(pred)` IS `filter(pred).next()` — same predicate, same
//! short-circuit, one fewer adapter. Span-preserving (T6.4): the whole
//! `next()` call's byte range is replaced by `<recv-of-filter>.find(<pred>)`,
//! each fragment being the ORIGINAL source text of the matched node.

use super::helpers::is_postfix_safe;
use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File};

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
        if let Some(replacement) = self.try_find(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_find(&self, m: &ExprMethodCall) -> Option<String> {
        if !m.attrs.is_empty() || m.turbofish.is_some() || m.method != "next" {
            return None;
        }
        if !m.args.is_empty() {
            return None;
        }
        let Expr::MethodCall(filter) = &*m.receiver else {
            return None; // receiver isn't a method call
        };
        if !filter.attrs.is_empty() || filter.turbofish.is_some() || filter.method != "filter" {
            return None;
        }
        if filter.args.len() != 1 {
            return None;
        }
        let recv = src(self.source, &filter.receiver)?;
        let pred = src(self.source, &filter.args[0])?;
        let recv = if is_postfix_safe(&filter.receiver) {
            recv
        } else {
            format!("({recv})")
        };
        Some(format!("{recv}.find({pred})"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_filter_next_to_find() {
        let input = "fn f(xs: &Vec<u8>) -> Option<&u8> { xs.iter().filter(|v| **v > 1).next() }";
        let out = propose_fix("filter_next", input).expect("matched");
        assert!(
            out.contains("xs.iter().find(|v| **v > 1)"),
            "find not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn bare_next_unchanged() {
        let input = "fn f(xs: &Vec<u8>) -> Option<&u8> { xs.iter().next() }";
        let out = propose_fix("filter_next", input);
        assert!(
            out.is_none(),
            "next on non-filter receiver must not fire: {out:?}"
        );
    }

    #[test]
    fn filter_without_next_unchanged() {
        let input = "fn f(xs: &Vec<u8>) -> usize { xs.iter().filter(|v| **v > 1).count() }";
        let out = propose_fix("filter_next", input);
        assert!(out.is_none(), "filter without next must not fire: {out:?}");
    }
}

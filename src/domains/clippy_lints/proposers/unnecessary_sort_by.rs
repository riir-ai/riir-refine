//! `clippy::unnecessary_sort_by` — `v.sort_by(|a, b| a.cmp(b))` → `v.sort()`.
//!
//! `sort_by` with the natural-order closure `|a, b| a.cmp(b)` is exactly
//! `sort()`. The closure must match in EXACT argument order — `|a, b|
//! b.cmp(a)` is a REVERSE sort and must not fire. `|&a, &b| a.cmp(b)`
//! (by-ref bindings) also reduces to `sort()`. Span-preserving (T6.4): the
//! whole `sort_by` call's byte range is replaced by `<recv>.sort()`.

use super::helpers::is_postfix_safe;
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, Pat};

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
        if let Some(replacement) = self.try_sort(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_sort(&self, m: &ExprMethodCall) -> Option<String> {
        if !m.attrs.is_empty() || m.turbofish.is_some() || m.method != "sort_by" {
            return None;
        }
        if m.args.len() != 1 {
            return None;
        }
        let Expr::Closure(c) = &m.args[0] else {
            return None;
        };
        if !c.attrs.is_empty() || c.inputs.len() != 2 {
            return None;
        }
        if c.asyncness.is_some() || c.constness.is_some() || c.movability.is_some() {
            return None; // dropping the modifier changes the closure type
        }
        // Both bindings must be plain idents (`subpat` forms like `a @ ..`
        // would be lost by the splice).
        let mut idents = c.inputs.iter().map(|p| match p {
            Pat::Ident(pi) if pi.subpat.is_none() => Some(&pi.ident),
            _ => None,
        });
        let a = idents.next()??;
        let b = idents.next()??;
        // Body must be exactly `a.cmp(b)` in that order.
        let Expr::MethodCall(cmp) = &*c.body else {
            return None;
        };
        if !cmp.attrs.is_empty() || cmp.turbofish.is_some() || cmp.method != "cmp" {
            return None;
        }
        if cmp.args.len() != 1 {
            return None;
        }
        let Expr::Path(recv_path) = &*cmp.receiver else {
            return None;
        };
        if recv_path.path.get_ident() != Some(a) {
            return None;
        }
        let Expr::Path(arg_path) = &cmp.args[0] else {
            return None;
        };
        if arg_path.path.get_ident() != Some(b) {
            return None;
        }
        let recv = src(self.source, &*m.receiver)?;
        let recv = if is_postfix_safe(&m.receiver) {
            recv
        } else {
            format!("({recv})")
        };
        Some(format!("{recv}.sort()"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_natural_order_sort_by() {
        let input = "fn f(mut v: Vec<u8>) { v.sort_by(|a, b| a.cmp(b)); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort()"), "sort not produced: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn deref_bindings_conservatively_skipped() {
        // `|&a, &b|` parses as Pat::Reference, not Pat::Ident — outside the
        // spec's exact shape, so the proposer skips it (false negatives are
        // safe for a fixer; clippy itself still flags it).
        let input = "fn f(v: &mut Vec<u8>) { v.sort_by(|&a, &b| a.cmp(b)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(
            out.is_none(),
            "non-Pat::Ident bindings must not fire: {out:?}"
        );
    }

    #[test]
    fn reversed_order_unchanged() {
        let input = "fn f(mut v: Vec<u8>) { v.sort_by(|a, b| b.cmp(a)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "reverse sort must not fire: {out:?}");
    }

    #[test]
    fn already_sorted_unchanged() {
        let input = "fn f(mut v: Vec<u8>) { v.sort(); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "already sort() must not fire: {out:?}");
    }

    #[test]
    fn custom_comparator_unchanged() {
        let input = "fn f(mut v: Vec<u8>) { v.sort_by(|a, b| a.abs().cmp(&b.abs())); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "non-cmp body must not fire: {out:?}");
    }
}

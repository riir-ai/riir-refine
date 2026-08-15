//! `clippy::map_unwrap_or` — `x.map(f).unwrap_or(d)` → `x.map_or(d, f)`.
//!
//! `Option::map_or(default, f)` is the fused form of `map(f).unwrap_or(d)`:
//! both evaluate `d` eagerly and `f` lazily, so the rewrite preserves
//! semantics for ANY default/mapper shapes (no purity requirement).
//! Span-preserving (T6.4): the whole `unwrap_or` call's byte range is
//! replaced by `<recv-of-map>.map_or(<D>, <F>)`, each fragment being the
//! ORIGINAL source text of the matched node.

use super::helpers::is_postfix_safe;
use super::{edit, src, SourceEdit};
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
        if let Some(replacement) = self.try_map_or(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_map_or(&self, m: &ExprMethodCall) -> Option<String> {
        if !m.attrs.is_empty() || m.turbofish.is_some() || m.method != "unwrap_or" {
            return None;
        }
        if m.args.len() != 1 {
            return None;
        }
        let Expr::MethodCall(map) = &*m.receiver else {
            return None; // receiver isn't a method call
        };
        if !map.attrs.is_empty() || map.turbofish.is_some() || map.method != "map" {
            return None;
        }
        if map.args.len() != 1 {
            return None;
        }
        let recv = src(self.source, &*map.receiver)?;
        let default = src(self.source, &m.args[0])?;
        let mapper = src(self.source, &map.args[0])?;
        let recv = if is_postfix_safe(&map.receiver) {
            recv
        } else {
            format!("({recv})")
        };
        Some(format!("{recv}.map_or({default}, {mapper})"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn fuses_map_unwrap_or() {
        let input = "fn f(x: Option<u8>) -> u8 { x.map(|v| v + 1).unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or(0, |v| v + 1)"),
            "map_or not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn chain_receiver_preserved() {
        let input = "fn f(v: Vec<u8>) -> u8 { v.first().map(|x| *x + 1).unwrap_or(9) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("v.first().map_or(9, |x| *x + 1)"),
            "chain receiver text not preserved: {out}"
        );
    }

    #[test]
    fn non_map_receiver_unchanged() {
        let input = "fn f(x: Option<u8>) -> u8 { x.unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "non-map receiver must not fire: {out:?}");
    }

    #[test]
    fn other_method_receiver_unchanged() {
        let input = "fn f(x: Option<u8>) -> u8 { x.filter(|v| *v > 0).unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "filter receiver must not fire: {out:?}");
    }
}

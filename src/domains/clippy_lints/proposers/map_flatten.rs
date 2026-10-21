//! `clippy::map_flatten` — `it.map(f).flatten()` → `it.flat_map(f)`
//! (Batch 155 / Plan 146 — the frontier-lead mining pack; family fixture
//! verified to fire exactly this lint under all+pedantic+nursery, 2026-09-18).
//!
//! `Iterator::flat_map(f)` IS `map(f).flatten()`: `Map`'s `Item` is exactly
//! `f`'s return value and both adapters flatten through the same
//! `IntoIterator` bound, so the rewrite is semantics-identical whenever both
//! compile (real clippy: `MachineApplicable`).
//!
//! Pattern: a no-arg `.flatten()` call whose receiver is a single-arg
//! `.map(..)` call. The WHOLE flatten call's span is replaced by
//! `<recv>.flat_map(<arg>)` rebuilt from ORIGINAL source fragments — the
//! receiver is parenthesized when not postfix-safe (the `filter_next`
//! technique).
//!
//! Guards: attrs/turbofish on either link refuse (a `flatten::<..>()` is
//! type-annotated — the rewrite would drop the annotation); `.map()` with
//! any arg count ≠ 1 refuses; a `flatten` with args refuses (different
//! method entirely).

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
        if let Some(replacement) = self.try_flat_map(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_flat_map(&self, m: &ExprMethodCall) -> Option<String> {
        if !m.attrs.is_empty() || m.turbofish.is_some() || m.method != "flatten" {
            return None;
        }
        if !m.args.is_empty() {
            return None;
        }
        let Expr::MethodCall(map) = &*m.receiver else {
            return None;
        };
        if !map.attrs.is_empty() || map.turbofish.is_some() || map.method != "map" {
            return None;
        }
        if map.args.len() != 1 {
            return None;
        }
        let recv = src(self.source, &*map.receiver)?;
        let f = src(self.source, &map.args[0])?;
        let recv = if is_postfix_safe(&map.receiver) {
            recv
        } else {
            format!("({recv})")
        };
        Some(format!("{recv}.flat_map({f})"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn fuses_map_flatten_into_flat_map() {
        let input =
            "fn word_lines(xs: &[String]) -> Vec<&str> { xs.iter().map(|x| x.split(' ')).flatten().collect() }";
        let out = propose_fix("map_flatten", input).expect("matched");
        assert!(
            out.contains("xs.iter().flat_map(|x| x.split(' ')).collect()"),
            "flat_map not produced: {out}"
        );
        assert!(!out.contains(".flatten()"), "flatten must be gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn bare_flatten_untouched() {
        let input = "fn go(xs: Vec<Vec<u8>>) -> Vec<u8> { xs.into_iter().flatten().collect() }";
        let out = propose_fix("map_flatten", input);
        assert!(
            out.is_none(),
            "flatten without map receiver must not fire: {out:?}"
        );
    }

    #[test]
    fn map_without_flatten_untouched() {
        let input = "fn go(xs: &[u8]) -> Vec<u8> { xs.iter().map(|x| x + 1).collect() }";
        let out = propose_fix("map_flatten", input);
        assert!(out.is_none(), "map without flatten must not fire: {out:?}");
    }

    #[test]
    fn turbofished_flatten_refused() {
        let input =
            "fn go(xs: &[u8]) -> Vec<u8> { xs.iter().map(|x| vec![x]).flatten::<Vec<u8>>().flatten().collect() }";
        let out = propose_fix("map_flatten", input);
        assert!(out.is_none(), "turbofished flatten must refuse: {out:?}");
    }

    #[test]
    fn non_postfix_receiver_parenthesized() {
        // A binary receiver needs parens when the method call is rebuilt.
        let input = "fn go(a: u8, b: u8) -> Vec<u8> { (if a > b { vec![a] } else { vec![b] }).map(|x| vec![x]).flatten().next().unwrap() }";
        // The `if` receiver: rebuilt as `(if ... { } else { }).flat_map(..)`.
        let out = propose_fix("map_flatten", input).expect("matched");
        assert!(
            out.contains("}).flat_map(|x| vec![x])"),
            "non-postfix receiver must be parenthesized: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn map_path_fn_arg_fuses() {
        let input =
            "fn go(xs: &[String]) -> Vec<&str> { xs.iter().map(String::as_str).flatten().collect() }";
        let out = propose_fix("map_flatten", input).expect("matched");
        assert!(
            out.contains("xs.iter().flat_map(String::as_str).collect()"),
            "path fn arg must fuse: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }
}

//! `clippy::filter_map_bool_then` — `xs.filter_map(|x| p(x).then(|| b(x)))`
//! → `xs.filter(|&x| p(x)).map(|x| b(x))`.
//!
//! On by default in clippy 1.98 (probe crate, 2026-09-13); real clippy's
//! suggestion is machine-applicable and produces exactly this shape. Span-
//! preserving (T6.4): the whole `filter_map(..)` call (receiver included) is
//! replaced by the two-adapter chain, each fragment being the ORIGINAL source
//! text of the matched node.
//!
//! Probe-mapped trigger surface (rustc 1.98.1, 2026-09-13):
//! - fires on `filter_map(|x| <bool expr>.then(|| <expr or block>))` — the
//!   predicate may be ANY expression (a bare `flag` path fires; a method call
//!   `x.leading_zeros() > 2` fires), and the `then` closure body may be a
//!   block (`|| { let y = x * 2; y + 1 }` fires);
//! - `then_some(..)` is SILENT (probed both by-value and String shapes) —
//!   the eager-argument twin changes evaluation semantics, so it stays out;
//! - real clippy is type-aware and only fires when the emitted `|&x|` filter
//!   pattern binds a `Copy` item (String items are silent — probed). syn
//!   cannot see the item type: this proposer fires structurally and rides
//!   the `--verify` compile gate — a non-`Copy` item fails E0507 at CHECK
//!   level (not just clippy typeck), so the gate is not blind to it (the
//!   `manual_is_multiple_of` ambiguous-`{integer}` lesson does not apply).
//!
//! Semantics: `filter(p).map(b)` evaluates `p` once per item and `b` once
//! per surviving item — exactly what `filter_map(|x| p(x).then(|| b(x)))`
//! did. Identical by construction, independent of types.

use super::helpers::is_postfix_safe;
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprClosure, ExprMethodCall, File, Pat};

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
        if let Some(replacement) = self.try_split(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_split(&self, m: &ExprMethodCall) -> Option<String> {
        if !m.attrs.is_empty() || m.turbofish.is_some() || m.method != "filter_map" {
            return None;
        }
        if m.args.len() != 1 {
            return None;
        }
        let outer = plain_bare_closure(&m.args[0])?;
        // The binding must be a plain by-value ident (`|x|`) — `|mut x|`,
        // `|ref x|`, `|&x|` and tuple/wildcard pats have no mechanical
        // `|&x|` translation and stay silent.
        if outer.inputs.len() != 1 {
            return None;
        }
        let Pat::Ident(p) = &outer.inputs[0] else {
            return None;
        };
        if p.by_ref.is_some() || p.subpat.is_some() || p.mutability.is_some() {
            return None;
        }
        let name = p.ident.to_string();
        // The outer closure body must be the BARE `<bool expr>.then(..)`
        // call — a block body (`|x| { p.then(|| b) }`) does not match here
        // and stays silent (probed shape: only the expression form fires).
        let Expr::MethodCall(then) = &*outer.body else {
            return None;
        };
        if !then.attrs.is_empty() || then.turbofish.is_some() || then.method != "then" {
            return None;
        }
        if then.args.len() != 1 {
            return None;
        }
        let inner = plain_bare_closure(&then.args[0])?;
        if !inner.inputs.is_empty() {
            return None;
        }
        let pred_text = src(self.source, &then.receiver)?;
        // The inner body may be an expression OR a block (probe shape N) —
        // either way it is re-emitted as the ORIGINAL source text.
        let body_text = src(self.source, &inner.body)?;
        let recv = src(self.source, &m.receiver)?;
        let recv = if is_postfix_safe(&m.receiver) {
            recv
        } else {
            format!("({recv})")
        };
        Some(format!(
            "{recv}.filter(|&{name}| {pred_text}).map(|{name}| {body_text})"
        ))
    }
}

/// A closure with no attributes, no `move`/`static`/`async` — the shape
/// both the outer `filter_map` argument and the inner `then` argument must
/// have (`move` is `capture` in syn — `movability` is `static`).
fn plain_bare_closure(e: &Expr) -> Option<&ExprClosure> {
    let Expr::Closure(c) = e else {
        return None;
    };
    if c.attrs.is_empty() && c.movability.is_none() && c.capture.is_none() && c.asyncness.is_none()
    {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn splits_filter_map_bool_then() {
        let input = "fn kept(xs: Vec<i32>) -> Vec<i32> { xs.into_iter().filter_map(|x| (x > 0).then(|| x * 2)).collect() }";
        let out = propose_fix("filter_map_bool_then", input).expect("matched");
        assert!(
            out.contains("filter(|&x| (x > 0)).map(|x| x * 2)"),
            "no split: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn method_predicate_fires() {
        // Probe shape O: Copy item + method-call predicate.
        let input = "fn f(xs: Vec<u32>) -> Vec<u32> { xs.into_iter().filter_map(|x| (x.leading_zeros() > 2).then(|| x * 2)).collect() }";
        let out = propose_fix("filter_map_bool_then", input).expect("matched");
        assert!(
            out.contains("filter(|&x| (x.leading_zeros() > 2))"),
            "method predicate must fire: {out}"
        );
    }

    #[test]
    fn binding_free_predicate_fires() {
        // Probe shape K: predicate does not mention the binding.
        let input = "fn f(xs: Vec<i32>, flag: bool) -> Vec<i32> { xs.into_iter().filter_map(|x| flag.then(|| x + 1)).collect() }";
        let out = propose_fix("filter_map_bool_then", input).expect("matched");
        assert!(
            out.contains("filter(|&x| flag).map(|x| x + 1)"),
            "bare-flag predicate must fire: {out}"
        );
    }

    #[test]
    fn block_closure_body_fires() {
        // Probe shape N: `then(|| { .. })` block body.
        let input = "fn f(xs: Vec<i32>) -> Vec<i32> { xs.into_iter().filter_map(|x| (x > 0).then(|| { let y = x * 2; y + 1 })).collect() }";
        let out = propose_fix("filter_map_bool_then", input).expect("matched");
        assert!(
            out.contains("map(|x| { let y = x * 2; y + 1 })"),
            "block body must survive verbatim: {out}"
        );
    }

    #[test]
    fn chained_receiver_preserved() {
        // Probe shape M: adapters chained AFTER filter_map stay untouched.
        let input = "fn f(xs: Vec<i32>) -> Vec<i32> { xs.into_iter().filter_map(|x| (x % 2 == 0).then(|| x / 2)).map(|x| x + 1).collect() }";
        let out = propose_fix("filter_map_bool_then", input).expect("matched");
        assert!(
            out.contains(".map(|x| x + 1).collect()"),
            "tail chain must survive: {out}"
        );
    }

    #[test]
    fn then_some_silent() {
        // Probe shapes I/S: then_some is silent in real clippy (eager arg).
        let input = "fn f(xs: Vec<i32>) -> Vec<i32> { xs.into_iter().filter_map(|x| (x > 0).then_some(x * 2)).collect() }";
        let out = propose_fix("filter_map_bool_then", input);
        assert!(out.is_none(), "then_some must not fire: {out:?}");
    }

    #[test]
    fn non_closure_arg_silent() {
        let input =
            "fn f(xs: Vec<i32>) -> Vec<i32> { xs.into_iter().filter_map(to_thing).collect() }";
        let out = propose_fix("filter_map_bool_then", input);
        assert!(out.is_none(), "non-closure arg must not fire: {out:?}");
    }

    #[test]
    fn move_closure_silent() {
        let input = "fn f(xs: Vec<i32>) -> Vec<i32> { xs.into_iter().filter_map(move |x| (x > 0).then(|| x)).collect() }";
        let out = propose_fix("filter_map_bool_then", input);
        assert!(out.is_none(), "move closure must not fire: {out:?}");
    }

    #[test]
    fn mut_binding_silent() {
        let input = "fn f(xs: Vec<Vec<i32>>) -> Vec<i32> { xs.into_iter().filter_map(|mut x| (x.len() == 1).then(|| x.pop())).collect() }";
        let out = propose_fix("filter_map_bool_then", input);
        assert!(out.is_none(), "mut binding must not fire: {out:?}");
    }

    #[test]
    fn plain_filter_map_if_else_silent() {
        // The if/else Some/None form is `manual_filter_map`'s territory,
        // not this lint's (probed silent in real clippy).
        let input = "fn f(xs: Vec<i32>) -> Vec<i32> { xs.into_iter().filter_map(|x| if x > 0 { Some(x * 2) } else { None }).collect() }";
        let out = propose_fix("filter_map_bool_then", input);
        assert!(out.is_none(), "if-else form must not fire: {out:?}");
    }

    #[test]
    fn block_bodied_outer_closure_silent() {
        // Only the expression-bodied outer closure has a mechanical split.
        let input = "fn f(xs: Vec<i32>) -> Vec<i32> { xs.into_iter().filter_map(|x| { (x > 0).then(|| x) }).collect() }";
        let out = propose_fix("filter_map_bool_then", input);
        assert!(out.is_none(), "block body must not fire: {out:?}");
    }
}

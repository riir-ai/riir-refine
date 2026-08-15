//! `clippy::unnecessary_lazy_evaluations` — `x.unwrap_or_else(|| 5)` →
//! `x.unwrap_or(5)`.
//!
//! Lazy-eval constructors (`unwrap_or_else` / `ok_or_else` / `or_else`) called
//! with a zero-parameter closure whose body is pure can use the eager twin:
//! the closure defers evaluation of an expression that has no side effects to
//! defer. Span-preserving (T6.4): the whole method call's byte range is
//! replaced by `<recv>.<eager>(<body>)`, each fragment being the ORIGINAL
//! source text of the matched node. `move` closures are accepted — a pure
//! body moves nothing, so `.unwrap_or(x)` uses it identically.

use super::helpers::{is_postfix_safe, is_pure_expr};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File};

/// Lazy constructor → eager twin.
const EAGER_TWINS: &[(&str, &str)] = &[
    ("unwrap_or_else", "unwrap_or"),
    ("ok_or_else", "ok_or"),
    ("or_else", "or"),
];

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
        if let Some(replacement) = self.try_eager(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_eager(&self, m: &ExprMethodCall) -> Option<String> {
        if !m.attrs.is_empty() || m.turbofish.is_some() || m.args.len() != 1 {
            return None; // attrs/turbofish would be dropped by the splice
        }
        let eager = EAGER_TWINS
            .iter()
            .find(|(lazy, _)| m.method == *lazy)
            .map(|(_, eager)| *eager)?;
        let Expr::Closure(c) = &m.args[0] else {
            return None;
        };
        if !c.inputs.is_empty() || !c.attrs.is_empty() {
            return None; // takes params / attrs would be dropped
        }
        if c.asyncness.is_some()
            || c.constness.is_some()
            || c.movability.is_some()
            || c.lifetimes.is_some()
        {
            return None; // dropping these modifiers changes the closure type
        }
        if !is_pure_expr(&c.body) {
            return None; // side effects must stay lazily evaluated
        }
        let recv = src(self.source, &*m.receiver)?;
        let body = src(self.source, &*c.body)?;
        let recv = if is_postfix_safe(&m.receiver) {
            recv
        } else {
            format!("({recv})")
        };
        Some(format!("{recv}.{eager}({body})"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn rewrites_pure_lazy_default() {
        let input = "fn f(x: Option<u8>) -> u8 { x.unwrap_or_else(|| 5) }";
        let out = propose_fix("unnecessary_lazy_evaluations", input).expect("matched");
        assert!(
            out.contains("x.unwrap_or(5)"),
            "eager twin not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn rewrites_binary_body_preserving_text() {
        let input = "fn f(x: Option<u8>) -> Result<u8, u16> { x.ok_or_else(|| 1 + 1) }";
        let out = propose_fix("unnecessary_lazy_evaluations", input).expect("matched");
        assert!(
            out.contains("x.ok_or(1 + 1)"),
            "body text not preserved: {out}"
        );
    }

    #[test]
    fn move_closure_accepted() {
        let input = "fn f(x: Option<u8>, y: u8) -> u8 { x.unwrap_or_else(move || y) }";
        let out = propose_fix("unnecessary_lazy_evaluations", input).expect("matched");
        assert!(
            out.contains("x.unwrap_or(y)"),
            "pure move body must fire: {out}"
        );
    }

    #[test]
    fn impure_body_unchanged() {
        let input =
            "fn compute() -> u8 { 1 }\nfn f(x: Option<u8>) -> u8 { x.unwrap_or_else(|| compute()) }";
        let out = propose_fix("unnecessary_lazy_evaluations", input);
        assert!(out.is_none(), "impure closure body must not fire: {out:?}");
    }

    #[test]
    fn closure_with_param_unchanged() {
        let input = "fn f(x: Option<u8>, e: u8) -> u8 { x.unwrap_or_else(|_| e) }";
        let out = propose_fix("unnecessary_lazy_evaluations", input);
        assert!(
            out.is_none(),
            "parameterized closure must not fire: {out:?}"
        );
    }

    #[test]
    fn already_eager_unchanged() {
        let input = "fn f(x: Option<u8>) -> u8 { x.unwrap_or(5) }";
        let out = propose_fix("unnecessary_lazy_evaluations", input);
        assert!(out.is_none(), "eager call must not fire: {out:?}");
    }
}

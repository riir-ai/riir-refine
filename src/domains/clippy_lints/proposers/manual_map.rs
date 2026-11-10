//! `clippy::manual_map` — `match opt { Some(v) => Some(f(v)), None => None }` →
//! `opt.map(|v| f(v))`.
//!
//! Span-preserving (T6.4): the whole match expression's byte range is
//! replaced by `<scrutinee>.map(|<binding>| <inner>)`, each fragment being
//! the ORIGINAL source text of the matched node. The scrutinee is wrapped
//! in parens when its expression form doesn't bind tighter than `.`
//! (the old `parse_quote!` joiner could silently regroup `a + b` into
//! `a + b.map(..)`).
//!
//! Control-flow guard (the riir-chain chainspec.rs E0277 lesson): the
//! rewrite moves the Some-arm body into a NEW closure. `?`/`return`/
//! `break`/`continue`/`await`/`yield` in the body change propagation or
//! fail to compile there (`?` propagated through the enclosing fn in
//! match-arm position → E0277 in closure position; `return` silently
//! returns from the closure instead). Real clippy's `manual_map` refuses
//! these bodies — probe-verified 1.97: the `?`-in-arm shape stays silent,
//! the clean shape fires.

use super::helpers::is_postfix_safe;
use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{Expr, ExprMatch, File, Pat};

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
    fn visit_expr_match(&mut self, m: &'ast ExprMatch) {
        syn::visit::visit_expr_match(self, m);
        if let Some(replacement) = self.try_manual_map(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_manual_map(&self, m: &ExprMatch) -> Option<String> {
        if m.arms.len() != 2 || !m.attrs.is_empty() {
            return None;
        }
        if m.arms.iter().any(|a| !a.attrs.is_empty()) {
            return None; // arm attrs would be lost by the splice
        }
        // Find which arm is the Some + which is the None.
        let (some_idx, none_idx) = match (is_some_pat(&m.arms[0].pat), is_none_pat(&m.arms[1].pat))
        {
            (true, true) => (0usize, 1usize),
            _ => match (is_none_pat(&m.arms[0].pat), is_some_pat(&m.arms[1].pat)) {
                (true, true) => (1usize, 0usize),
                _ => return None,
            },
        };
        // none arm body must be the bare `None`.
        if !is_none_expr(&m.arms[none_idx].body) {
            return None;
        }
        // The Some-arm body moves into a NEW closure: any control flow that
        // depended on the enclosing fn (`?`, `return`, `break`, `continue`,
        // `await`, `yield`) breaks there — refuse (real clippy does too).
        if m.arms
            .iter()
            .any(|a| body_has_closure_unsafe_control_flow(&a.body))
        {
            return None;
        }
        // some arm body must be `Some(<expr>)`.
        let some_arm = &m.arms[some_idx];
        let inner: &Expr = match &*some_arm.body {
            Expr::Call(c) => {
                if !matches!(&*c.func, Expr::Path(p) if p.path.is_ident("Some")) {
                    return None;
                }
                if c.args.len() != 1 {
                    return None;
                }
                &c.args[0]
            }
            _ => return None,
        };
        // some arm pat must be `Some(<binding>)`.
        let binding: &syn::PatIdent = match &some_arm.pat {
            Pat::TupleStruct(ts) if ts.path.is_ident("Some") && ts.elems.len() == 1 => {
                match &ts.elems[0] {
                    Pat::Ident(pi) => pi,
                    _ => return None,
                }
            }
            _ => return None,
        };
        let scrutinee = src(self.source, &m.expr)?;
        let binding = src(self.source, binding)?;
        let inner = src(self.source, inner)?;
        let scrutinee = if is_postfix_safe(&m.expr) {
            scrutinee
        } else {
            format!("({scrutinee})")
        };
        Some(format!("{scrutinee}.map(|{binding}| {inner})"))
    }
}

fn is_some_pat(p: &Pat) -> bool {
    matches!(p, Pat::TupleStruct(ts) if ts.path.is_ident("Some") && ts.elems.len() == 1)
}

fn is_none_pat(p: &Pat) -> bool {
    // `None` can parse as Pat::Path (when treated as a unit-variant path),
    // Pat::TupleStruct(empty) (when treated as a unit-struct pattern), or
    // Pat::Ident (syn ambiguity — `None` is also a valid binding name).
    // We check the token text to disambiguate.
    matches!(p, Pat::Path(pp) if pp.path.is_ident("None"))
        || matches!(p, Pat::TupleStruct(ts) if ts.path.is_ident("None") && ts.elems.is_empty())
        || matches!(p, Pat::Ident(pi) if pi.ident == "None")
}

fn is_none_expr(e: &Expr) -> bool {
    matches!(e, Expr::Path(pp) if pp.path.is_ident("None"))
}

/// Does this expression contain control flow that cannot survive being
/// spliced into a fresh closure body? Checked on BOTH arms (the None arm
/// is a bare `None` literal by construction, so this is future-proofing).
struct ClosureUnsafeFlow {
    hit: bool,
}

impl<'ast> Visit<'ast> for ClosureUnsafeFlow {
    fn visit_expr(&mut self, e: &'ast Expr) {
        if matches!(
            e,
            Expr::Try(_)
                | Expr::TryBlock(_)
                | Expr::Await(_)
                | Expr::Return(_)
                | Expr::Break(_)
                | Expr::Continue(_)
                | Expr::Yield(_)
        ) {
            self.hit = true;
        }
        syn::visit::visit_expr(self, e);
    }
}

fn body_has_closure_unsafe_control_flow(e: &Expr) -> bool {
    let mut v = ClosureUnsafeFlow { hit: false };
    v.visit_expr(e);
    v.hit
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_match_to_map() {
        let input = "fn to_text(x: Option<u8>) -> Option<String> {\n    match x {\n        Some(v) => Some(v.to_string()),\n        None => None,\n    }\n}";
        let out = propose_fix("manual_map", input).expect("matched");
        assert!(
            out.contains("x.map(|v| v.to_string())"),
            "map not produced: {out}"
        );
    }

    #[test]
    fn non_postfix_scrutinee_gets_parens() {
        // A deref scrutinee must NOT become `*r.map(..)` (deref of the map).
        let input = "fn f(r: &Option<u8>) -> Option<u8> { match *r { Some(v) => Some(v + 1), None => None, } }";
        let out = propose_fix("manual_map", input).expect("matched");
        assert!(
            out.contains("(*r).map(|v| v + 1)"),
            "deref scrutinee must be parenthesized: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn method_scrutinee_no_parens() {
        let input = "fn f(x: Option<u8>) -> Option<u8> { match x.clone() { Some(v) => Some(v), None => None, } }";
        let out = propose_fix("manual_map", input).expect("matched");
        assert!(
            out.contains("x.clone().map(|v| v)"),
            "postfix-safe scrutinee needs no parens: {out}"
        );
    }

    #[test]
    fn leaves_non_match_unchanged() {
        let input = "fn to_text(x: Option<u8>) -> Option<String> { x.map(|v| v.to_string()) }";
        let out = propose_fix("manual_map", input);
        assert!(out.is_none(), "non-match touched: {out:?}");
    }

    #[test]
    fn leaves_two_arm_match_not_some_none() {
        let input = "fn go(x: u8) -> u8 { match x { 1 => 2, _ => 3 } }";
        let out = propose_fix("manual_map", input);
        assert!(out.is_none(), "non-manual-map match touched: {out:?}");
    }

    #[test]
    fn none_first_arm_order_accepted() {
        let input = "fn f(x: Option<u8>) -> Option<u8> { match x { None => None, Some(v) => Some(v + 1), } }";
        let out = propose_fix("manual_map", input).expect("matched");
        assert!(out.contains("x.map(|v| v + 1)"), "{out}");
    }

    #[test]
    fn try_in_arm_body_is_refused() {
        // The riir-chain chainspec.rs production shape: `?` inside the
        // Some-arm body propagated through the enclosing fn in match-arm
        // position — moving it into a closure is E0277. Real clippy 1.97
        // stays silent on this shape (probe-verified).
        let input = "fn f(raw: Option<&str>) -> Result<Option<[u8; 32]>, E> {\n    let k = match raw {\n        None => None,\n        Some(hex) => Some(parse_hex32(hex).ok_or_else(|| E)?),\n    };\n    Ok(k)\n}";
        let out = propose_fix("manual_map", input);
        assert!(out.is_none(), "try-in-arm body must be refused: {out:?}");
    }

    #[test]
    fn return_in_arm_body_is_refused() {
        // `return` in a closure returns from the CLOSURE, not the fn —
        // a silent semantic change, worse than the E0277.
        let input = "fn f(x: Option<u8>) -> Option<u8> { match x { None => None, Some(v) => Some({ if v > 1 { return None; } v + 1 }) } }";
        let out = propose_fix("manual_map", input);
        assert!(out.is_none(), "return-in-arm body must be refused: {out:?}");
    }

    #[test]
    fn await_in_arm_body_is_refused() {
        let input = "async fn f(x: Option<Foo>) -> Option<Bar> { match x { None => None, Some(v) => Some(v.load().await) } }";
        let out = propose_fix("manual_map", input);
        assert!(out.is_none(), "await-in-arm body must be refused: {out:?}");
    }
}

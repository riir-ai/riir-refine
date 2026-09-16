//! `clippy::partialeq_to_none` — `x == None` → `x.is_none()`,
//! `x != None` → `x.is_some()` (Batch 150; Bench 094 default-level
//! evidence: 9 firings / 3 crates).
//!
//! ## Probe matrix (real clippy 0.1.98)
//!
//! FIRES — every binary comparison against a literal `None`:
//! `x == None` (→ `is_none()`), `x != None` (→ `is_some()`), the
//! reference form `r == &None` (→ `r.is_none()`), and the qualified
//! `x == Option::None`. SILENT — `x.is_none()` itself, and a type with a
//! custom `PartialEq<Option<T>>` impl (real clippy is type-aware: it
//! only fires when the operand type IS `Option`).
//!
//! Tier: VERIFY. The rewrite is type-blind — a non-`Option` receiver
//! with a custom `PartialEq<Option>` impl has no `is_none()` (E0599) —
//! but that failure is check-visible, so the `--fix --verify` compile
//! gate auto-reverts it. This is the VERIFY tier's exact design
//! (the `redundant_closure` demotion precedent). Real clippy's suggestion
//! is `MachineApplicable`, so the P21 oracle lane also consumes it for the
//! shapes this proposer's receiver-shape guard refuses.
//!
//! Receiver safety: the receiver text is spliced verbatim; postfix-shaped
//! receivers (path/call/method/field/index/macro/paren/await) go bare,
//! everything else is parenthesized (`(a + b).is_none()`) — a method call
//! binds tighter than any binary operator, so the parenthesized form is
//! always precedence-correct.

use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{BinOp, Expr, File};

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
    fn visit_expr_binary(&mut self, e: &'ast syn::ExprBinary) {
        syn::visit::visit_expr_binary(self, e);
        let recv = match (&e.op, is_none_literal(&e.left), is_none_literal(&e.right)) {
            (BinOp::Eq(_), true, false) => Some(("is_none", &e.right)),
            (BinOp::Eq(_), false, true) => Some(("is_none", &e.left)),
            (BinOp::Ne(_), true, false) => Some(("is_some", &e.right)),
            (BinOp::Ne(_), false, true) => Some(("is_some", &e.left)),
            _ => None,
        };
        let Some((method, recv)) = recv else {
            return;
        };
        // `None == None` — the receiver is itself a None literal; skip
        // (valid but pointless, and not the debt class).
        if is_none_literal(recv) {
            return;
        }
        let Some(text) = src(self.source, recv) else {
            return;
        };
        let receiver = if is_postfix(recv) {
            text
        } else {
            format!("({text})")
        };
        self.edits.push(edit(e, format!("{receiver}.{method}()")));
    }
}

/// A literal `None`: the bare path `None`, the qualified `Option::None`,
/// or the reference form `&None` (the probe's `r == &None` shape).
fn is_none_literal(e: &Expr) -> bool {
    match e {
        Expr::Path(p) => is_none_path(&p.path),
        Expr::Reference(r) => match &*r.expr {
            Expr::Path(p) => is_none_path(&p.path),
            _ => false,
        },
        _ => false,
    }
}

fn is_none_path(path: &syn::Path) -> bool {
    let segs: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    (segs.len() == 1 && segs[0] == "None")
        || (segs.len() == 2 && segs[0] == "Option" && segs[1] == "None")
}

/// Postfix-shaped expressions may be method-call receivers bare; a
/// method call binds tighter than any binary operator, so these never
/// need parentheses.
fn is_postfix(e: &Expr) -> bool {
    matches!(
        e,
        Expr::Path(_)
            | Expr::MethodCall(_)
            | Expr::Call(_)
            | Expr::Field(_)
            | Expr::Index(_)
            | Expr::Macro(_)
            | Expr::Paren(_)
            | Expr::Await(_)
    )
}

#[cfg(test)]
mod tests {

    fn fix(code: &str) -> Option<String> {
        super::super::propose_fix("partialeq_to_none", code)
    }

    #[test]
    fn fires_on_every_probe_firing_shape() {
        for (buggy, fixed) in [
            (
                "fn f(x: Option<u32>) -> bool { x == None }",
                "fn f(x: Option<u32>) -> bool { x.is_none() }",
            ),
            (
                "fn f(x: Option<u32>) -> bool { x != None }",
                "fn f(x: Option<u32>) -> bool { x.is_some() }",
            ),
            (
                "fn f(r: &Option<u32>) -> bool { r == &None }",
                "fn f(r: &Option<u32>) -> bool { r.is_none() }",
            ),
            (
                "fn f(x: Option<u32>) -> bool { x == Option::None }",
                "fn f(x: Option<u32>) -> bool { x.is_none() }",
            ),
            (
                "fn f(x: Option<u32>) -> bool { None == x }",
                "fn f(x: Option<u32>) -> bool { x.is_none() }",
            ),
            (
                "fn f(a: Option<u32>, b: Option<u32>) -> bool { a + b == None }",
                "fn f(a: Option<u32>, b: Option<u32>) -> bool { (a + b).is_none() }",
            ),
        ] {
            assert_eq!(fix(buggy).as_deref(), Some(fixed), "case: {buggy}");
        }
    }

    #[test]
    fn silent_on_control_and_none_none() {
        for code in [
            "fn f(x: Option<u32>) -> bool { x.is_none() }",
            "fn f(x: Option<u32>) -> bool { x == Some(3) }",
        ] {
            assert_eq!(fix(code), None, "must stay silent: {code}");
        }
    }
}

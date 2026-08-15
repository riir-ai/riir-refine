//! `clippy::identity_op` — `x * 1`, `1 * y`, `x / 1`, `n - 0`, `n + 0i32`
//! → the non-literal operand (Issue 016).
//!
//! FLOAT SAFETY (the load-bearing spec): IEEE-754 makes `x + 0.0` NOT an
//! identity (`-0.0 + 0.0 = +0.0`), while `x - 0.0`, `x * 1.0` and `x / 1.0`
//! ARE identities for every float. Unsuffixed integer literals can infer
//! to floats, so the ops split:
//!
//! | Op | Shape | Unsuffixed literal? |
//! |---|---|---|
//! | Add | `x + 0` / `0 + x` | NO — the zero must carry an integer suffix (`0i32`) |
//! | Sub | `x - 0` (rhs only) | yes (float-safe) |
//! | Mul | `x * 1` / `1 * x` | yes (float-safe) |
//! | Div | `x / 1` (rhs only) | yes (float-safe) |
//!
//! Only `Lit::Int` literals ever match (`0.0`/`1.0` are `Lit::Float` and
//! are rejected wholesale). The rewrite replaces the whole binary expr
//! with the ORIGINAL source text of the surviving operand.

use super::helpers::int_lit;
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{BinOp, Expr, ExprBinary, File};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector { source, edits: Vec::new() };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_binary(&mut self, b: &'ast ExprBinary) {
        syn::visit::visit_expr_binary(self, b);
        let Some(kept) = identity_operand(b) else { return };
        if let Some(text) = src(self.source, kept) {
            self.edits.push(edit(b, text));
        }
    }
}

/// The operand that survives the identity rewrite, if `b` is an identity op.
fn identity_operand(b: &ExprBinary) -> Option<&Expr> {
    match b.op {
        BinOp::Add(_) => {
            // NOT float-safe unsuffixed (−0.0 + 0.0 = +0.0): the zero must
            // be provably integral via a non-empty integer suffix.
            if is_zero(&b.right, true) {
                Some(&b.left)
            } else if is_zero(&b.left, true) {
                Some(&b.right)
            } else {
                None
            }
        }
        BinOp::Sub(_) => {
            // `x - 0.0` IS a float identity (−0.0 − 0.0 = −0.0); rhs only —
            // `0 - x` is negation, not identity.
            if is_zero(&b.right, false) { Some(&b.left) } else { None }
        }
        BinOp::Mul(_) => {
            if is_one(&b.right) {
                Some(&b.left)
            } else if is_one(&b.left) {
                Some(&b.right)
            } else {
                None
            }
        }
        BinOp::Div(_) => {
            // rhs only — `1 / x` is not identity (and can trap at x == 0).
            if is_one(&b.right) { Some(&b.left) } else { None }
        }
        _ => None,
    }
}

/// Integer literal with value `digits`; `require_suffix` gates the
/// float-unsafe Add case (an unsuffixed `0` can infer to `0.0`).
fn is_int_with(e: &Expr, digits: &str, require_suffix: bool) -> bool {
    int_lit(e).is_some_and(|(d, suffix)| d == digits && (!require_suffix || !suffix.is_empty()))
}

fn is_zero(e: &Expr, require_suffix: bool) -> bool {
    is_int_with(e, "0", require_suffix)
}

fn is_one(e: &Expr) -> bool {
    is_int_with(e, "1", false)
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn mul_by_one_rhs() {
        let out = propose_fix("identity_op", "fn f(x: i32) -> i32 { x * 1 }").expect("matched");
        assert_eq!(out, "fn f(x: i32) -> i32 { x }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn mul_by_one_lhs() {
        let out = propose_fix("identity_op", "fn f(y: i32) -> i32 { 1 * y }").expect("matched");
        assert_eq!(out, "fn f(y: i32) -> i32 { y }");
    }

    #[test]
    fn add_suffixed_zero() {
        let out =
            propose_fix("identity_op", "fn f(n: i32) -> i32 { n + 0i32 }").expect("matched");
        assert_eq!(out, "fn f(n: i32) -> i32 { n }");
    }

    #[test]
    fn sub_zero() {
        let out = propose_fix("identity_op", "fn f(n: i32) -> i32 { n - 0 }").expect("matched");
        assert_eq!(out, "fn f(n: i32) -> i32 { n }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn div_by_one() {
        let out = propose_fix("identity_op", "fn f(x: i32) -> i32 { x / 1 }").expect("matched");
        assert_eq!(out, "fn f(x: i32) -> i32 { x }");
    }

    #[test]
    fn unsuffixed_add_zero_unchanged() {
        // ADVERSARIAL: `x + 0` could be `x + 0.0` under inference, and
        // −0.0 + 0.0 = +0.0 — NOT an identity. Must not fire.
        let out = propose_fix("identity_op", "fn f(x: f64) -> f64 { x + 0 }");
        assert!(out.is_none(), "unsuffixed `+ 0` must not fire: {out:?}");
    }

    #[test]
    fn unsuffixed_leading_add_zero_unchanged() {
        let out = propose_fix("identity_op", "fn f(x: f64) -> f64 { 0 + x }");
        assert!(out.is_none(), "unsuffixed `0 +` must not fire: {out:?}");
    }

    #[test]
    fn add_one_unchanged() {
        let out = propose_fix("identity_op", "fn f(x: i32) -> i32 { x + 1 }");
        assert!(out.is_none(), "`x + 1` is not an identity: {out:?}");
    }

    #[test]
    fn add_float_half_unchanged() {
        let out = propose_fix("identity_op", "fn f(x: f64) -> f64 { x + 0.5 }");
        assert!(out.is_none(), "float operand must not fire: {out:?}");
    }

    #[test]
    fn zero_minus_x_unchanged() {
        // `0 - x` is negation, not identity.
        let out = propose_fix("identity_op", "fn f(x: i32) -> i32 { 0 - x }");
        assert!(out.is_none(), "`0 - x` must not fire: {out:?}");
    }
}

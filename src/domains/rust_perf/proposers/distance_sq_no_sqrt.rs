//! `distance-sq-no-sqrt` — `(dx*dx + dy*dy).sqrt() < r` →
//! `(dx*dx + dy*dy) < r * r` (compare against the squared threshold).
//!
//! Pattern: a comparison (`<` `>` `<=` `>=`, either operand order) where
//! one side is `.sqrt()` (method call, no args, no turbofish) over a
//! **sum-of-squares** receiver — an add-chain whose every leaf term is a
//! product `x * x` with TEXTUALLY-IDENTICAL operands (provably
//! non-negative) — and the other side is a **pure threshold** expression:
//! a path or a literal (parens unwrapped). The receiver text is reused
//! verbatim (parens preserved) and the threshold text is duplicated as
//! `t * t`. Bench 038/B53: ParetoDominant 1.21-1.24x — the fixed form
//! vectorizes; sqrt-then-compare serializes on the transcendental even
//! where aarch64 f32 sqrt is a single NEON instruction.
//!
//! ## Gates
//! - **Sum-of-squares receiver** (the `distance-sq` of the rule id): the
//!   sqrt argument must be an add-chain of `x * x` leaves with
//!   textually-identical operands — `dx * dx + dy * dy` fires,
//!   `dx * dy` (not a square) and `a * b + c * c` (mixed) skip. The sum
//!   of squares is >= 0 by construction, which is what makes squaring
//!   order-preserving (sqrt is monotone on non-negatives). A bare path
//!   receiver (`d2.sqrt()`) has no syntactic non-negativity proof — skip.
//! - **Pure threshold**: the non-sqrt side may be a path or a literal
//!   only (parens unwrapped) — the rewrite DUPLICATES its text (`t * t`),
//!   so method calls / index exprs (possible side effects, double
//!   evaluation) skip. A negative literal is syntactically an
//!   `Expr::Unary`, so it never matches — only non-negative thresholds
//!   fire by construction.
//! - **Inline shape only**: the sqrt result must feed the comparison
//!   directly. `let d = (..).sqrt(); if d < r` (flow-separated) is out
//!   of scope — the rule's recorded guard "keep sqrt when the VALUE is
//!   consumed" holds automatically (a let-bound sqrt has no comparison
//!   match here).
//! - `sqrt(a) < sqrt(b)` (both sides sqrt) skips — that is a different
//!   rewrite (no squaring); comparisons inside macro token streams are
//!   invisible to syn and skip.
//!
//! ## Honest semantic caveats (the rule's recorded guards, restated)
//! - **Negative threshold diverges**: for `r < 0` the original is
//!   always-false (sqrt >= 0) while the rewrite can be true (`E < r*r`).
//!   Distance/radius thresholds are non-negative by domain meaning; a
//!   caller that can pass a negative threshold should keep the sqrt form.
//! - **1-ulp boundary band**: `fl(r * r)` may round below the true `r²`,
//!   so `E` within one ulp of the (rounded) boundary can flip. Exact
//!   boundary behavior is not preserved — the standard tolerance of the
//!   compare-squared idiom; keep sqrt where the razor edge is
//!   load-bearing (e.g. collision determinism pinned to sqrt).
//! - NaN propagates equivalently (NaN compares false in both forms);
//!   overflow to inf is order-equivalent on both sides.
//! - f32 and f64 both fire (`.sqrt()` exists on both; monotone on both).

use syn::visit::Visit;
use syn::{BinOp, Expr, ExprBinary, File};

use super::{edit, src, SourceEdit};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut scan = CmpScan {
        source,
        edits: Vec::new(),
    };
    scan.visit_file(file);
    scan.edits
}

struct CmpScan<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for CmpScan<'_> {
    fn visit_expr(&mut self, e: &'ast Expr) {
        if let Expr::Binary(bin) = e {
            if matches!(
                bin.op,
                BinOp::Lt(_) | BinOp::Gt(_) | BinOp::Le(_) | BinOp::Ge(_)
            ) {
                if let Some(replacement) = self.try_rewrite(bin) {
                    self.edits.push(edit(bin, replacement));
                    return; // replaced span — do not recurse into it
                }
            }
        }
        syn::visit::visit_expr(self, e);
    }
}

impl CmpScan<'_> {
    /// `sqrt_side OP threshold` → `sos OP t * t`;
    /// `threshold OP sqrt_side` → `t * t OP sos`.
    fn try_rewrite(&self, bin: &ExprBinary) -> Option<String> {
        let left_sqrt = sqrt_sos_text(&bin.left, self.source);
        let right_sqrt = sqrt_sos_text(&bin.right, self.source);
        let (sos, threshold, threshold_on_left) = match (left_sqrt, right_sqrt) {
            (Some(sos), None) => (sos, &bin.right, false),
            (None, Some(sos)) => (sos, &bin.left, true),
            // Neither side qualifies, or both are sqrt (out of scope).
            _ => return None,
        };
        let t = pure_threshold_text(threshold, self.source)?;
        let op = op_str(&bin.op)?;
        if threshold_on_left {
            // `t * t OP sos` — `*` binds tighter than the comparison, so
            // this parses as `(t * t) OP sos`, never `t * (t OP sos)`.
            Some(format!("{t} * {t} {op} {sos}"))
        } else {
            Some(format!("{sos} {op} {t} * {t}"))
        }
    }
}

/// The receiver text of `.sqrt()` over a sum-of-squares, else `None`.
fn sqrt_sos_text(e: &Expr, source: &str) -> Option<String> {
    let Expr::MethodCall(mc) = e else { return None };
    if mc.method != "sqrt" || !mc.args.is_empty() || mc.turbofish.is_some() {
        return None;
    }
    if !is_sum_of_squares(&mc.receiver, source) {
        return None;
    }
    src(source, &mc.receiver)
}

/// Add-chain of `x * x` leaves (parens unwrapped at every level). A
/// single `x * x` term qualifies — the one-term degenerate of the chain.
fn is_sum_of_squares(e: &Expr, source: &str) -> bool {
    match e {
        Expr::Paren(p) => is_sum_of_squares(&p.expr, source),
        Expr::Binary(b) => match b.op {
            BinOp::Add(_) => {
                is_sum_of_squares(&b.left, source) && is_sum_of_squares(&b.right, source)
            }
            BinOp::Mul(_) => match (src(source, &b.left), src(source, &b.right)) {
                (Some(l), Some(r)) => l.trim() == r.trim(),
                _ => false,
            },
            _ => false,
        },
        _ => false,
    }
}

/// Original text of a pure threshold: path or literal (parens
/// unwrapped). Everything else — unary, method call, index, binary —
/// skips (the rewrite would duplicate evaluation).
fn pure_threshold_text(e: &Expr, source: &str) -> Option<String> {
    match e {
        Expr::Path(_) | Expr::Lit(_) => src(source, e),
        Expr::Paren(p) => pure_threshold_text(&p.expr, source),
        _ => None,
    }
}

fn op_str(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::Lt(_) => Some("<"),
        BinOp::Gt(_) => Some(">"),
        BinOp::Le(_) => Some("<="),
        BinOp::Ge(_) => Some(">="),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    fn parses(out: &str) {
        assert!(syn::parse_file(out).is_ok(), "output does not parse: {out}");
    }

    /// The corpus fixture shape (variable threshold, trailing comment).
    #[test]
    fn corpus_fixture_rewrites() {
        let out = propose_fix(
            "distance-sq-no-sqrt",
            "fn in_range(dx: f32, dy: f32, r: f32) -> bool {\n    (dx * dx + dy * dy).sqrt() \
             < r // sqrt only to compare\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("(dx * dx + dy * dy) < r * r"), "{out}");
        // The trailing comment survives (outside the replaced span).
        assert!(out.contains("// sqrt only to compare"), "{out}");
        parses(&out);
    }

    /// The eval-span shape (named threshold `leash`).
    #[test]
    fn eval_span_shape_fires() {
        let out = propose_fix(
            "distance-sq-no-sqrt",
            "pub fn pet_within_leash(dx: f32, dy: f32, leash: f32) -> bool {\n    \
             (dx * dx + dy * dy).sqrt() < leash\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("< leash * leash"), "{out}");
        parses(&out);
    }

    #[test]
    fn reversed_operands_fire() {
        let out = propose_fix(
            "distance-sq-no-sqrt",
            "fn near(cx: f32, cy: f32, r: f64) -> bool {\n    r > (cx * cx + cy * cy).sqrt()\n}",
        )
        .expect("pattern matched");
        // `*` binds tighter than `>` — `r * r > (..)` parses as `(r*r) > (..)`.
        assert!(out.contains("r * r > (cx * cx + cy * cy)"), "{out}");
        parses(&out);
    }

    #[test]
    fn literal_threshold_fires() {
        let out = propose_fix(
            "distance-sq-no-sqrt",
            "fn f(x: f32, y: f32) -> bool { (x * x + y * y).sqrt() < 100.0 }",
        )
        .expect("pattern matched");
        assert!(out.contains("< 100.0 * 100.0"), "{out}");
        parses(&out);
    }

    #[test]
    fn le_ge_variants_fire() {
        let out = propose_fix(
            "distance-sq-no-sqrt",
            "fn f(x: f32, r: f32) -> bool { (x * x).sqrt() <= r }",
        )
        .expect("pattern matched");
        assert!(out.contains("(x * x) <= r * r"), "{out}");
        parses(&out);

        let out = propose_fix(
            "distance-sq-no-sqrt",
            "fn g(x: f32, r: f32) -> bool { (x * x).sqrt() >= r }",
        )
        .expect("pattern matched");
        assert!(out.contains(">= r * r"), "{out}");
        parses(&out);
    }

    #[test]
    fn three_term_chain_fires() {
        let out = propose_fix(
            "distance-sq-no-sqrt",
            "fn f(x: f64, y: f64, z: f64, r: f64) -> bool {\n    \
             (x * x + y * y + z * z).sqrt() < r\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("(x * x + y * y + z * z) < r * r"), "{out}");
        parses(&out);
    }

    #[test]
    fn nonsquare_product_skips() {
        assert!(propose_fix(
            "distance-sq-no-sqrt",
            "fn f(dx: f32, dy: f32, r: f32) -> bool { (dx * dy).sqrt() < r }",
        )
        .is_none());
    }

    #[test]
    fn mixed_chain_skips() {
        // `a * b + c * c` — the `a * b` term is not a square.
        assert!(propose_fix(
            "distance-sq-no-sqrt",
            "fn f(a: f32, b: f32, c: f32, r: f32) -> bool { (a * b + c * c).sqrt() < r }",
        )
        .is_none());
    }

    #[test]
    fn method_call_threshold_skips() {
        // Double evaluation of a side-effecting call — never rewrite.
        assert!(propose_fix(
            "distance-sq-no-sqrt",
            "fn f(x: f32, y: f32) -> bool { (x * x + y * y).sqrt() < get_radius() }",
        )
        .is_none());
    }

    #[test]
    fn path_receiver_skips() {
        // No syntactic non-negativity proof for `d2`.
        assert!(propose_fix(
            "distance-sq-no-sqrt",
            "fn f(d2: f32, r: f32) -> bool { d2.sqrt() < r }",
        )
        .is_none());
    }

    #[test]
    fn negative_literal_skips() {
        // `-1.0` parses as Expr::Unary — never a pure literal.
        assert!(propose_fix(
            "distance-sq-no-sqrt",
            "fn f(x: f32) -> bool { (x * x).sqrt() < -1.0 }",
        )
        .is_none());
    }

    #[test]
    fn both_sides_sqrt_skips() {
        assert!(propose_fix(
            "distance-sq-no-sqrt",
            "fn f(a: f32, b: f32) -> bool { (a * a).sqrt() < (b * b).sqrt() }",
        )
        .is_none());
    }

    #[test]
    fn let_bound_sqrt_skips() {
        // Flow-separated sqrt — the recorded guard ("keep sqrt when the
        // VALUE is consumed") applies; no inline comparison to match.
        assert!(propose_fix(
            "distance-sq-no-sqrt",
            "fn f(dx: f32, dy: f32, r: f32) -> bool {\n    let d = (dx * dx + dy * dy).sqrt();\n    \
             d < r\n}",
        )
        .is_none());
    }

    #[test]
    fn nested_in_if_condition_fires() {
        // Compound squares — `(px - tx) * (px - tx)` has identical operand
        // texts, so the chain qualifies even though each square is compound.
        let out = propose_fix(
            "distance-sq-no-sqrt",
            "fn f(px: f32, py: f32, tx: f32, ty: f32, snap: f32) {\n    \
             if ((px - tx) * (px - tx) + (py - ty) * (py - ty)).sqrt() < snap {\n        \
             println!(\"snap\");\n    }\n}",
        )
        .expect("identical-operand squares fire even when compound");
        assert!(out.contains("< snap * snap"), "{out}");
        parses(&out);
    }
}

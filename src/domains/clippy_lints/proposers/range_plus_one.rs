//! `clippy::range_plus_one` — `0..n + 1` → `0..=n` (Issue 016).
//!
//! Pattern: a half-open range whose end is `<E> + 1` (or `1 + <E>`) where
//! the `1` is an INTEGER literal — a float `1.0` never fires (a different
//! value class, not just a different bound). Parenthesized ends (Issue 029:
//! `0..(depth + 1)`, the weaver.rs miss shape) are unwrapped first. The whole
//! range node is replaced by `<start>..=<E>` built from the ORIGINAL source
//! fragments; an open-start range rewrites to `..=E`.

use super::helpers::int_lit;
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{BinOp, Expr, ExprBinary, ExprRange, File, RangeLimits};

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
    fn visit_expr_range(&mut self, r: &'ast ExprRange) {
        syn::visit::visit_expr_range(self, r);
        if !matches!(r.limits, RangeLimits::HalfOpen(_)) {
            return; // `..=` is already inclusive
        }
        let Some(end_expr) = r.end.as_deref() else {
            return;
        };
        // Issue 029: `0..(n + 1)` parses the end as `Expr::Paren` wrapping
        // the Binary — unwrap paren layers before the Binary match (real
        // clippy fires on the parenthesized shape; weaver.rs:2642 miss).
        let mut unwrapped = end_expr;
        while let Expr::Paren(p) = unwrapped {
            unwrapped = &p.expr;
        }
        let Expr::Binary(end) = unwrapped else {
            return;
        };
        let Some(new_end) = plus_one_end(end) else {
            return;
        };
        let start_text = match r.start.as_deref() {
            Some(start) => match src(self.source, start) {
                Some(text) => format!("{text}..="),
                None => return, // broken span bookkeeping — skip, don't mis-splice
            },
            None => "..=".to_string(),
        };
        let Some(end_text) = src(self.source, new_end) else {
            return;
        };
        self.edits.push(edit(r, format!("{start_text}{end_text}")));
    }
}

/// In `<lhs> + <rhs>`, the operand that survives when exactly one side is
/// the integer literal `1` (any suffix; `Lit::Int` only, so `1.0` never
/// matches). Both-sides-literal sums (`1 + 1`) conservatively decline.
fn plus_one_end(b: &ExprBinary) -> Option<&Expr> {
    if !matches!(b.op, BinOp::Add(_)) {
        return None;
    }
    match (int_lit(&b.left), int_lit(&b.right)) {
        (Some((digits, _)), None) if digits == "1" => Some(&b.right),
        (None, Some((digits, _))) if digits == "1" => Some(&b.left),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn half_open_plus_one_becomes_inclusive() {
        let out = propose_fix(
            "range_plus_one",
            "fn f(n: u32) { let r = 0..n+1; let _ = r; }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(n: u32) { let r = 0..=n; let _ = r; }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn literal_one_on_the_left_also_fires() {
        let out = propose_fix(
            "range_plus_one",
            "fn f(n: u32) { let r = 0..1+n; let _ = r; }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(n: u32) { let r = 0..=n; let _ = r; }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn open_start_range_rewrites_without_start() {
        let out = propose_fix(
            "range_plus_one",
            "fn f(n: u32) { let r = ..n+1; let _ = r; }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(n: u32) { let r = ..=n; let _ = r; }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn named_end_variable_keeps_fragment() {
        let out = propose_fix(
            "range_plus_one",
            "fn f(limit: usize) { let r = 0..limit+1; let _ = r; }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(limit: usize) { let r = 0..=limit; let _ = r; }");
    }

    #[test]
    fn parenthesized_plus_one_end_fires() {
        // Issue 029: the weaver.rs:2642 miss shape — a paren-wrapped `+1`
        // end parses as Expr::Paren, which the Binary match used to reject.
        let out = propose_fix(
            "range_plus_one",
            "fn f(depth: usize) { let v: Vec<u8> = (0..(depth + 1)).map(|_| 1u8).collect(); let _ = v; }",
        )
        .expect("pattern matched");
        assert!(
            out.contains("(0..=depth)"),
            "paren end not unwrapped: {out}"
        );
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn doubly_parenthesized_end_fires() {
        let out = propose_fix(
            "range_plus_one",
            "fn f(n: u32) { let r = 0..((n + 1)); let _ = r; }",
        )
        .expect("pattern matched");
        assert!(out.contains("0..=n"), "nested parens not unwrapped: {out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn parenthesized_non_plus_one_unchanged() {
        // Unwrapping parens must not loosen the +1 requirement.
        let out = propose_fix(
            "range_plus_one",
            "fn f(n: u32) { let r = 0..(n + 2); let _ = r; }",
        );
        assert!(out.is_none(), "(n+2) must not fire: {out:?}");
    }

    #[test]
    fn parenthesized_mul_end_unchanged() {
        // `(j + 1) * n` — the end is a Mul, not `<E> + 1`; real clippy does
        // not fire this shape (caddtree_budget.rs:952 is NOT a miss).
        let out = propose_fix(
            "range_plus_one",
            "fn f(j: usize, n: usize) { let s = &wte[j * n..(j + 1) * n]; let _ = s; }",
        );
        assert!(out.is_none(), "mul end must not fire: {out:?}");
    }

    #[test]
    fn already_inclusive_unchanged() {
        let out = propose_fix(
            "range_plus_one",
            "fn f(n: u32) { let r = 0..=n; let _ = r; }",
        );
        assert!(out.is_none(), "inclusive range changed: {out:?}");
    }

    #[test]
    fn plain_half_open_unchanged() {
        let out = propose_fix(
            "range_plus_one",
            "fn f(n: u32) { let r = 0..n; let _ = r; }",
        );
        assert!(out.is_none(), "non-plus-one range changed: {out:?}");
    }

    #[test]
    fn plus_two_unchanged() {
        let out = propose_fix(
            "range_plus_one",
            "fn f(n: u32) { let r = 0..n+2; let _ = r; }",
        );
        assert!(out.is_none(), "n+2 is not an inclusive rewrite: {out:?}");
    }

    #[test]
    fn float_one_unchanged() {
        // `1.0` is a float literal — never a `..=` bound rewrite.
        let out = propose_fix(
            "range_plus_one",
            "fn f(n: f64) { let r = 0.0..n+1.0; let _ = r; }",
        );
        assert!(out.is_none(), "float literal must not fire: {out:?}");
    }
}

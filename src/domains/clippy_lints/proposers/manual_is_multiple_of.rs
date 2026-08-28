//! `clippy::manual_is_multiple_of` — `x % d == 0` → `x.is_multiple_of(d)`;
//! `!=` → `!`; the reversed `0 == x % d` normalizes to the same call. The
//! divisor expression is spliced verbatim (probed vs real clippy 1.97:
//! literal, non-literal, and complex divisors all fire with the fix keeping
//! the divisor expression). Paren layers around the `%` are unwrapped
//! (`(x % 4) == 0` de-parens — the method call binds tightest).
//!
//! VERIFY-tier (Issue 015 class, [`super::VERIFY_LINTS`]): parse-only cannot
//! see dividend signedness, and `is_multiple_of` is UNSIGNED-ONLY (E0599 on
//! `i32`, probed) — the `--verify` compile gate auto-reverts any
//! signed-dividend break. Oracle boundary (probed vs clippy 1.97): real
//! clippy is SILENT on signed dividends (type-aware) — our proposals on
//! those shapes are backed by rustc instead; the method form
//! `(x % 4).eq(&0)` does not fire there (and is not an Eq/Ne binary here);
//! float operands never fire (no `is_multiple_of` on floats — literal
//! floats are skipped at proposal time). `is_multiple_of` needs Rust ≥ 1.87;
//! a consumer pinned older fails the verify gate, which is the sanctioned
//! revert path.
//!
//! Postfix guard: the shared [`is_postfix_safe`] EXCLUDES casts (P11 —
//! `x as u64.method()` parses the method into the TYPE position and fails),
//! so cast dividends parenthesize (`(x as u64).is_multiple_of(4)`; fixture
//! `unparenthesized_cast_dividend_parenthesizes`).
//!
//! Self-recursion guard (Issue 031 H1 class): `self % d == 0` inside
//! `fn is_multiple_of` would rewrite to a call of itself — infinite
//! recursion the compile gate is structurally blind to (it compiles fine).
//! Refused like `comparison_to_empty`'s `is_empty` twin.

use super::helpers::{int_lit, is_postfix_safe};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{BinOp, Expr, ExprBinary, ExprLit, Lit, File};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        fn_stack: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Innermost-last stack of enclosing function names (Issue 031 H1).
    fn_stack: Vec<String>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        self.fn_stack.push(i.sig.ident.to_string());
        syn::visit::visit_item_fn(self, i);
        self.fn_stack.pop();
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        self.fn_stack.push(i.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, i);
        self.fn_stack.pop();
    }

    fn visit_trait_item_fn(&mut self, i: &'ast syn::TraitItemFn) {
        self.fn_stack.push(i.sig.ident.to_string());
        syn::visit::visit_trait_item_fn(self, i);
        self.fn_stack.pop();
    }

    fn visit_expr_binary(&mut self, b: &'ast ExprBinary) {
        syn::visit::visit_expr_binary(self, b);
        let negated = match b.op {
            BinOp::Eq(_) => false,
            BinOp::Ne(_) => true,
            _ => return, // ordering comparisons mean something else
        };
        if !b.attrs.is_empty() {
            return;
        }
        // Which side is the modulo? Either order fires; `== 1` and friends
        // do not mean multiple-of.
        let rem = if let Some(r) = unwrap_mod(&b.left) {
            if is_zero(&b.right) {
                r
            } else {
                return;
            }
        } else if let Some(r) = unwrap_mod(&b.right) {
            if is_zero(&b.left) {
                r
            } else {
                return;
            }
        } else {
            return;
        };
        if !rem.attrs.is_empty() {
            return;
        }
        if is_float_lit(&rem.left) || is_float_lit(&rem.right) {
            return;
        }
        if let Some(replacement) = multiple_of_text(
            self.source,
            rem,
            negated,
            self.fn_stack.last().map(String::as_str),
        ) {
            self.edits.push(edit(b, replacement));
        }
    }
}

/// `<dividend>.is_multiple_of(<divisor>)` — parens on the dividend unless it
/// is postfix-safe (the shared helper's cast exclusion covers the
/// method-append hazard), the divisor verbatim (argument position needs no
/// guard), `!` prefix when the comparison was `!=`.
fn multiple_of_text(
    source: &str,
    rem: &ExprBinary,
    negated: bool,
    fn_top: Option<&str>,
) -> Option<String> {
    let dividend = src(source, &*rem.left)?;
    let divisor = src(source, &*rem.right)?;
    // Issue 031 H1: `self % d == 0` inside `fn is_multiple_of` rewrites to
    // an infinite self-call — refuse (different receivers still fire).
    if dividend == "self" && fn_top == Some("is_multiple_of") {
        return None;
    }
    let recv = if is_postfix_safe(&rem.left) {
        dividend
    } else {
        format!("({dividend})")
    };
    let prefix = if negated { "!" } else { "" };
    Some(format!("{prefix}{recv}.is_multiple_of({divisor})"))
}

/// Paren layers peeled to the first `%` binary — `(x % 4) == 0` and
/// `((x % 4)) == 0` both yield the inner binary (the replacement span is the
/// whole comparison, so the parens vanish with it).
fn unwrap_mod(e: &Expr) -> Option<&ExprBinary> {
    let mut cur = e;
    loop {
        match cur {
            Expr::Paren(p) => cur = &p.expr,
            Expr::Binary(b) if matches!(b.op, BinOp::Rem(_)) => return Some(b),
            _ => return None,
        }
    }
}

/// Is this the literal `0` (any integer suffix)?
fn is_zero(e: &Expr) -> bool {
    int_lit(e).is_some_and(|(digits, _)| digits == "0")
}

fn is_float_lit(e: &Expr) -> bool {
    matches!(e, Expr::Lit(ExprLit { lit: Lit::Float(_), .. }))
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_modulo_zero_check() {
        let input = "fn f(n: u64) -> bool { n % 4 == 0 }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(out.contains("n.is_multiple_of(4)"), "fix not produced: {out}");
        assert!(!out.contains("% 4"), "modulo remains: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn converts_inequality_with_negation() {
        let input = "fn f(n: u64) -> bool { n % 4 != 0 }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(
            out.contains("!n.is_multiple_of(4)"),
            "negated form not produced: {out}"
        );
    }

    #[test]
    fn converts_reversed_operands() {
        let input = "fn f(n: u64) -> bool { 0 == n % 4 }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(out.contains("n.is_multiple_of(4)"), "reversed form: {out}");
    }

    #[test]
    fn converts_reversed_negated() {
        let input = "fn f(n: u64) -> bool { 0 != n % 4 }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(out.contains("!n.is_multiple_of(4)"), "reversed negated: {out}");
    }

    #[test]
    fn paren_wrapped_modulo_de_parens() {
        let input = "fn f(n: u64) -> bool { (n % 4) == 0 }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(
            out.contains("n.is_multiple_of(4)"),
            "paren layer not unwrapped: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn suffixed_zero_fires() {
        let input = "fn f(n: u64) -> bool { n % 4 == 0usize }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(out.contains("n.is_multiple_of(4)"), "suffix zero: {out}");
    }

    #[test]
    fn binary_dividend_parenthesizes() {
        let input = "fn f(n: u64) -> bool { (n + 1) % 4 == 0 }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(
            out.contains("(n + 1).is_multiple_of(4)"),
            "binary dividend not parenthesized: {out}"
        );
    }

    #[test]
    fn unparenthesized_cast_dividend_parenthesizes() {
        // `n as u64` binds tighter than `%`, so the dividend IS the cast —
        // and a cast is not method-append-safe (`x as u64.method()` groups
        // the method into the type position).
        let input = "fn f(n: u32) -> bool { n as u64 % 4 == 0 }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(
            out.contains("(n as u64).is_multiple_of(4)"),
            "cast dividend not parenthesized: {out}"
        );
    }

    #[test]
    fn paren_cast_dividend_keeps_source_parens() {
        let input = "fn f(n: u32) -> bool { (n as u64) % 4 == 0 }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(
            out.contains("(n as u64).is_multiple_of(4)"),
            "paren-cast dividend: {out}"
        );
    }

    #[test]
    fn method_call_dividend_stays_postfix() {
        let input = "fn f(v: &[u8]) -> bool { v.len() % 4 == 0 }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(
            out.contains("v.len().is_multiple_of(4)"),
            "method dividend: {out}"
        );
    }

    #[test]
    fn non_literal_divisor_spliced_verbatim() {
        // Probed: real clippy keeps a non-literal divisor expression.
        let input = "fn f(n: u64, d: u64) -> bool { n % d == 0 }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(
            out.contains("n.is_multiple_of(d)"),
            "non-literal divisor: {out}"
        );
    }

    #[test]
    fn complex_divisor_spliced_verbatim() {
        // Span-verbatim splice: the source's own parens ride along (redundant
        // in argument position, harmless, and never re-grouped).
        let input = "fn f(n: u64) -> bool { n % (2 * 2) == 0 }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(
            out.contains("n.is_multiple_of((2 * 2))"),
            "complex divisor: {out}"
        );
    }

    #[test]
    fn nonzero_comparison_does_not_fire() {
        // `== 1` does not mean multiple-of.
        let input = "fn f(n: u64) -> bool { n % 4 == 1 }";
        let out = propose_fix("manual_is_multiple_of", input);
        assert!(out.is_none(), "non-zero comparison fired: {out:?}");
    }

    #[test]
    fn ordering_comparison_does_not_fire() {
        let input = "fn f(n: u64) -> bool { n % 4 > 0 }";
        let out = propose_fix("manual_is_multiple_of", input);
        assert!(out.is_none(), "ordering comparison fired: {out:?}");
    }

    #[test]
    fn method_form_does_not_fire() {
        // Real clippy is silent on the `.eq(&0)` form (probed); it is not an
        // Eq/Ne binary, so the proposer must stay silent too.
        let input = "fn f(n: u64) -> bool { (n % 4).eq(&0) }";
        let out = propose_fix("manual_is_multiple_of", input);
        assert!(out.is_none(), "method form fired: {out:?}");
    }

    #[test]
    fn float_dividend_does_not_fire() {
        // No `is_multiple_of` on floats — real clippy is type-aware silent.
        let input = "fn f(x: f64) -> bool { x % 4.0 == 0.0 }";
        let out = propose_fix("manual_is_multiple_of", input);
        assert!(out.is_none(), "float shape fired: {out:?}");
    }

    #[test]
    fn self_recursion_guard() {
        // Issue 031 H1 class: `self % 4 == 0` inside `fn is_multiple_of`
        // rewrites to a call of itself — infinite recursion the compile gate
        // cannot see (it compiles). Parse-valid fixture; need not compile.
        let input = "impl M {\n    fn is_multiple_of(&self) -> bool {\n        self % 4 == 0\n    }\n}";
        let out = propose_fix("manual_is_multiple_of", input);
        assert!(out.is_none(), "self-recursion shape must not fire: {out:?}");
    }

    #[test]
    fn self_receiver_outside_is_multiple_of_still_fires() {
        // Positive control: `self` dividend inside a differently-named fn.
        let input = "impl M {\n    fn check(&self) -> bool {\n        self % 4 == 0\n    }\n}";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(
            out.contains("self.is_multiple_of(4)"),
            "self receiver outside is_multiple_of lost: {out}"
        );
    }
}

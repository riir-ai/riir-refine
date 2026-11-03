//! `clippy::manual_is_multiple_of` — `x % d == 0` → `x.is_multiple_of(d)`;
//! `!=` → `!`; the reversed `0 == x % d` normalizes to the same call. The
//! divisor expression is spliced verbatim (probed vs real clippy 1.97:
//! literal, non-literal, and complex divisors all fire with the fix keeping
//! the divisor expression). Paren layers around the `%` are unwrapped
//! (`(x % 4) == 0` de-parens — the method call binds tightest).
//!
//! VERIFY-tier (Issue 015 class, [`super::VERIFY_LINTS`]): the dividend must
//! be PROVABLY unsigned at the syntax level. `is_multiple_of` is
//! UNSIGNED-ONLY, and the dangerous class is the AMBIGUOUS `{integer}`
//! dividend (bare range-loop vars, untyped lets, unsuffixed literals):
//! plain `cargo check` resolves those clean while CLIPPY's typeck raises
//! E0689 ("can't call method on ambiguous numeric type") — the check-only
//! `--verify` gate is structurally blind to it (the 2026-08-29 sweep
//! restored 8 such breaks by hand). The proposer therefore fires only on a
//! dividend whose type it can prove unsigned:
//!
//! - a cast to an unsigned type (`x as u64`),
//! - an unsigned-suffixed int literal,
//! - a `len()`/`capacity()` method call (usize by signature on std types;
//!   a pathological custom `len() -> i32` is E0599 under plain check —
//!   the verify gate still backs that class),
//! - an arithmetic binary with at least one provably-unsigned operand
//!   (the operands unify, so one typed side proves the result),
//! - a bare ident bound by a TYPED fn parameter or TYPED `let` whose type
//!   is an unsigned path (`u8`..`u128`, `usize`).
//!
//! Everything else (untyped loop vars, `self`, untyped literals, field
//! accesses, unknown method returns) stays SILENT. Oracle boundary (probed
//! vs clippy 1.97): real clippy is type-aware and only fires where it can
//! prove the type — this guard closes the same gap from the syntax side;
//! the residual divergence (safe-but-skipped shapes) is conservative, not
//! harmful. Float operands never fire (no `is_multiple_of` on floats —
//! literal floats are skipped at proposal time). `is_multiple_of` needs
//! Rust ≥ 1.87; a consumer pinned older fails the verify gate, which is the
//! sanctioned revert path.
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
use syn::{BinOp, Expr, ExprBinary, ExprLit, File, Lit};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        fn_stack: Vec::new(),
        unsigned_idents: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Innermost-last stack of enclosing function names (Issue 031 H1).
    fn_stack: Vec<String>,
    /// Innermost-last stack of per-fn ident sets whose type is PROVABLY an
    /// unsigned integer at the syntax level (typed fn parameters + typed
    /// `let` bindings). Guards the E0689 ambiguous-integer class — see the
    /// module doc.
    unsigned_idents: Vec<std::collections::HashSet<String>>,
}

/// Is this a path type naming an unsigned integer (`u8`..`u128`, `usize`)?
fn is_unsigned_path(ty: &syn::Type) -> bool {
    let syn::Type::Path(tp) = ty else {
        return false;
    };
    tp.path.get_ident().is_some_and(|id| {
        matches!(
            id.to_string().as_str(),
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
        )
    })
}

/// Collect the fn's provably-unsigned idents: typed parameters with unsigned
/// annotated types + typed `let` bindings anywhere in the body (the body-wide
/// scan is deliberately name-based — shadowing a typed-unsigned name with a
/// signed binding would false-fire, and that class is E0599 under plain
/// check, i.e. verify-backstopped; the E0689 class this guard exists for is
/// NOT check-visible).
fn collect_unsigned_idents(
    fn_item_sig: &syn::Signature,
    body: Option<&syn::Block>,
) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    for input in &fn_item_sig.inputs {
        if let syn::FnArg::Typed(pat) = input {
            if let syn::Pat::Ident(pi) = &*pat.pat {
                if is_unsigned_path(&pat.ty) {
                    set.insert(pi.ident.to_string());
                }
            }
        }
    }
    if let Some(body) = body {
        collect_unsigned_lets(&body.stmts, &mut set);
    }
    set
}

fn collect_unsigned_lets(stmts: &[syn::Stmt], set: &mut std::collections::HashSet<String>) {
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(local) => {
                // `let k: u64 = ..` — the annotation is part of the PATTERN
                // in syn 2.x (Local has no ty field).
                if let syn::Pat::Type(pt) = &local.pat {
                    if is_unsigned_path(&pt.ty) {
                        if let syn::Pat::Ident(pi) = &*pt.pat {
                            set.insert(pi.ident.to_string());
                        }
                    }
                }
            }
            syn::Stmt::Item(syn::Item::Fn(inner)) => {
                // Nested fns carry their own scopes; merge is conservative.
                let nested = collect_unsigned_idents(&inner.sig, Some(&inner.block));
                set.extend(nested);
            }
            _ => {}
        }
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        self.fn_stack.push(i.sig.ident.to_string());
        self.unsigned_idents
            .push(collect_unsigned_idents(&i.sig, Some(&i.block)));
        syn::visit::visit_item_fn(self, i);
        self.unsigned_idents.pop();
        self.fn_stack.pop();
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        self.fn_stack.push(i.sig.ident.to_string());
        self.unsigned_idents
            .push(collect_unsigned_idents(&i.sig, Some(&i.block)));
        syn::visit::visit_impl_item_fn(self, i);
        self.unsigned_idents.pop();
        self.fn_stack.pop();
    }

    fn visit_trait_item_fn(&mut self, i: &'ast syn::TraitItemFn) {
        self.fn_stack.push(i.sig.ident.to_string());
        let body = i.default.as_ref();
        self.unsigned_idents
            .push(collect_unsigned_idents(&i.sig, body));
        syn::visit::visit_trait_item_fn(self, i);
        self.unsigned_idents.pop();
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
        // E0689 guard: fire only on a PROVABLY unsigned dividend — the
        // ambiguous-`{integer}` class passes plain `cargo check` but fails
        // CLIPPY's typeck (check-only --verify is blind to it).
        let unsigned = self.unsigned_idents.last();
        if !dividend_provably_unsigned(&rem.left, unsigned) {
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
    let dividend = src(source, &rem.left)?;
    let divisor = src(source, &rem.right)?;
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

/// Methods whose return type is `usize` by signature across the std
/// collection/string surface (a pathological custom `len() -> i32` would be
/// E0599 under plain check — verify-backstopped, unlike the E0689 class).
fn is_known_unsigned_method(e: &Expr) -> bool {
    let Expr::MethodCall(m) = e else {
        return false;
    };
    matches!(m.method.to_string().as_str(), "len" | "capacity")
}

/// Can the dividend's type be proven unsigned from syntax alone? See the
/// module doc for the whitelist. Anything unproven stays silent — the
/// proposer must never emit the E0689-under-clippy shape.
fn dividend_provably_unsigned(
    e: &Expr,
    unsigned_idents: Option<&std::collections::HashSet<String>>,
) -> bool {
    match e {
        Expr::Paren(p) => dividend_provably_unsigned(&p.expr, unsigned_idents),
        Expr::Cast(c) => is_unsigned_path(&c.ty),
        Expr::Lit(l) => match &l.lit {
            Lit::Int(i) => {
                let suffix = i.suffix().trim_start_matches('_');
                matches!(suffix, "u8" | "u16" | "u32" | "u64" | "u128" | "usize")
            }
            _ => false,
        },
        Expr::MethodCall(_) => is_known_unsigned_method(e),
        // Arithmetic operands unify — one provably-unsigned side proves the
        // result (`n + 1` with `n: u64` is u64).
        Expr::Binary(b)
            if matches!(
                b.op,
                BinOp::Add(_) | BinOp::Sub(_) | BinOp::Mul(_) | BinOp::Div(_) | BinOp::Rem(_)
            ) =>
        {
            dividend_provably_unsigned(&b.left, unsigned_idents)
                || dividend_provably_unsigned(&b.right, unsigned_idents)
        }
        Expr::Path(p) => p
            .path
            .get_ident()
            .is_some_and(|id| unsigned_idents.is_some_and(|set| set.contains(&id.to_string()))),
        _ => false,
    }
}

fn is_float_lit(e: &Expr) -> bool {
    matches!(
        e,
        Expr::Lit(ExprLit {
            lit: Lit::Float(_),
            ..
        })
    )
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_modulo_zero_check() {
        let input = "fn f(n: u64) -> bool { n % 4 == 0 }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(
            out.contains("n.is_multiple_of(4)"),
            "fix not produced: {out}"
        );
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
        assert!(
            out.contains("!n.is_multiple_of(4)"),
            "reversed negated: {out}"
        );
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
        let input =
            "impl M {\n    fn is_multiple_of(&self) -> bool {\n        self % 4 == 0\n    }\n}";
        let out = propose_fix("manual_is_multiple_of", input);
        assert!(out.is_none(), "self-recursion shape must not fire: {out:?}");
    }

    #[test]
    fn self_receiver_outside_is_multiple_of_now_skipped() {
        // E0689 guard: `self`'s type is Self (unknown at the syntax level) —
        // `self % 4 == 0` compiles for signed Self but the rewrite breaks.
        // The old positive control is now a documented skip.
        let input = "impl M {\n    fn check(&self) -> bool {\n        self % 4 == 0\n    }\n}";
        let out = propose_fix("manual_is_multiple_of", input);
        assert!(
            out.is_none(),
            "untyped self receiver must not fire: {out:?}"
        );
    }

    #[test]
    fn self_cast_to_unsigned_still_fires() {
        // Positive control for the skip above: the cast proves the type.
        let input =
            "impl M {\n    fn check(&self) -> bool {\n        self.0 as u64 % 4 == 0\n    }\n}";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(
            out.contains("(self.0 as u64).is_multiple_of(4)"),
            "self-cast dividend lost: {out}"
        );
    }

    #[test]
    fn bare_loop_var_dividend_skipped() {
        // THE 2026-08-29 sweep break class: `i` over `0..n` is an ambiguous
        // `{integer}` — plain cargo check resolves it clean, CLIPPY raises
        // E0689, and the check-only --verify gate is blind. Must stay silent.
        let input = "fn run(n: usize) -> usize {\n    let mut c = 0;\n    for i in 0..n {\n        if i % 3 == 0 {\n            c += 1;\n        }\n    }\n    c\n}";
        let out = propose_fix("manual_is_multiple_of", input);
        assert!(out.is_none(), "ambiguous loop-var dividend fired: {out:?}");
    }

    #[test]
    fn untyped_let_dividend_skipped() {
        let input = "fn run() -> bool {\n    let k = 9;\n    k % 3 == 0\n}";
        let out = propose_fix("manual_is_multiple_of", input);
        assert!(out.is_none(), "untyped let dividend fired: {out:?}");
    }

    #[test]
    fn typed_let_dividend_fires() {
        let input = "fn run() -> bool {\n    let k: u64 = 9;\n    k % 3 == 0\n}";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(out.contains("k.is_multiple_of(3)"), "typed let lost: {out}");
    }

    #[test]
    fn unsuffixed_literal_dividend_skipped() {
        // `4 % 2 == 0` falls back to i32 — signed — the rewrite breaks.
        let input = "fn f() -> bool { 4 % 2 == 0 }";
        let out = propose_fix("manual_is_multiple_of", input);
        assert!(out.is_none(), "unsuffixed literal dividend fired: {out:?}");
    }

    #[test]
    fn suffixed_unsigned_literal_dividend_fires() {
        let input = "fn f() -> bool { 4u64 % 2 == 0 }";
        let out = propose_fix("manual_is_multiple_of", input).expect("matched");
        assert!(
            out.contains("4u64.is_multiple_of(2)"),
            "suffixed literal lost: {out}"
        );
    }

    #[test]
    fn field_access_dividend_skipped() {
        // Field types are unknown at the syntax level (parse-valid fixture).
        let input = "fn f(s: &S) -> bool { s.count % 4 == 0 }";
        let out = propose_fix("manual_is_multiple_of", input);
        assert!(out.is_none(), "field-access dividend fired: {out:?}");
    }
}

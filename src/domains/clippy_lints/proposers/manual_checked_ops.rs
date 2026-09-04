//! `clippy::manual_checked_ops` — `if n == 0 { v } else { total / n }` →
//! `total.checked_div(n).unwrap_or(v)`.
//!
//! Complexity lint (new in clippy 1.98, complexity group): a zero-guard
//! guards around a division is a hand-rolled `checked_div`. Real clippy fires ONLY
//! on unsigned integer divisions (the probe confirmed i32/i64/isize are
//! silent — for signed types `checked_div` additionally catches the
//! `MIN / -1` overflow, which is a behavior change, so clippy refuses) and
//! is suggestion-only (`MaybeIncorrect` — its `--fix` applied 0/17 on the
//! 2026-09-04 riir-dao run). Our proposer CAN see the else arm, which is the
//! batch's reason to exist.
//!
//! Tier: **VERIFY** (type-blind classes remain — see the module tail).
//!
//! Firing shapes (probe-mapped 1:1): `d == 0` / `0 == d` (division in the
//! else arm), `d != 0` / `d > 0` / `0 < d` (division in the then arm), in
//! expression or statement position, on `usize`/`u64`/`u32`. NOT fired by
//! clippy and not fired here: `%` (no `checked_rem` shape in the lint),
//! method-call divisors (`total / q.len()` — the divisor would be evaluated
//! twice), and provably-signed divisors (typed `i*`/`isize` params or lets —
//! our gate refuses them even though syn cannot see most types).
//!
//! Guards beyond the probe map:
//! - the divisor must be a `Path` or `Field` expression (double-evaluation
//!   purity; matches clippy's own refusal of `q.len()` divisors);
//! - both arms must be single-expression blocks;
//! - the division arm must be a `Div` binary (parens peeled) whose RHS is
//!   TEXTUALLY the divisor from the condition (`src` equality);
//! - no comments inside the if span (the whole-if replacement would
//!   displace them).
//!
//! Residual type-blindness (documented, verify-tier): a `Field` divisor of
//! unknown type could be signed — the rewrite then differs only in the
//! `MIN / -1` corner (original panics — division overflow panics in both
//! profiles — rewrite returns the guard value); a custom `Div` type has no
//! `checked_div` (E0599). The `--fix --verify` compile gate reverts the
//! E0599 class; the MIN/-1 corner is accepted as the lint's own intent (the
//! author's guard value is the natural continuation).

use super::helpers::{byte_range, drops_interior_comment, int_lit, is_postfix_safe};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{BinOp, Expr, ExprIf, ExprParen, ExprPath, File, Stmt};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        signed_idents: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Innermost-last stack of per-fn ident sets whose type is PROVABLY a
    /// SIGNED integer (typed params + typed lets). Mirrors the
    /// `manual_is_multiple_of` unsigned-set pattern inverted: we refuse the
    /// provably-signed class (real clippy never fires there) and allow the
    /// unknown class (field divisors — the common real shape).
    signed_idents: Vec<std::collections::HashSet<String>>,
}

fn is_signed_path(ty: &syn::Type) -> bool {
    let syn::Type::Path(tp) = ty else {
        return false;
    };
    tp.path
        .get_ident()
        .is_some_and(|id| matches!(id.to_string().as_str(), "i8" | "i16" | "i32" | "i64" | "i128" | "isize"))
}

fn collect_signed_idents(sig: &syn::Signature, body: Option<&syn::Block>) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    for input in &sig.inputs {
        if let syn::FnArg::Typed(pat) = input {
            if let syn::Pat::Ident(pi) = &*pat.pat {
                if is_signed_path(&pat.ty) {
                    set.insert(pi.ident.to_string());
                }
            }
        }
    }
    if let Some(body) = body {
        for stmt in &body.stmts {
            if let syn::Stmt::Local(local) = stmt {
                if let syn::Pat::Type(pt) = &local.pat {
                    if is_signed_path(&pt.ty) {
                        if let syn::Pat::Ident(pi) = &*pt.pat {
                            set.insert(pi.ident.to_string());
                        }
                    }
                }
            }
        }
    }
    set
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        self.signed_idents.push(collect_signed_idents(&i.sig, Some(&i.block)));
        syn::visit::visit_item_fn(self, i);
        self.signed_idents.pop();
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        self.signed_idents.push(collect_signed_idents(&i.sig, Some(&i.block)));
        syn::visit::visit_impl_item_fn(self, i);
        self.signed_idents.pop();
    }

    fn visit_trait_item_fn(&mut self, i: &'ast syn::TraitItemFn) {
        let body = i.default.as_ref();
        self.signed_idents.push(collect_signed_idents(&i.sig, body));
        syn::visit::visit_trait_item_fn(self, i);
        self.signed_idents.pop();
    }

    fn visit_expr_if(&mut self, e: &'ast ExprIf) {
        syn::visit::visit_expr_if(self, e);
        if let Some(replacement) = self.try_checked_div(e) {
            self.edits.push(edit(e, replacement));
        }
    }
}

/// Where the division arm sits, given the condition's operator.
#[derive(Clone, Copy, PartialEq)]
enum DivArm {
    Then,
    Else,
}

impl Collector<'_> {
    fn try_checked_div(&self, e: &ExprIf) -> Option<String> {
        if !e.attrs.is_empty() {
            return None;
        }
        let Expr::Binary(cond) = &*e.cond else {
            return None;
        };
        let (div_arm, divisor) = match cond.op {
            BinOp::Eq(_) => {
                let d = if is_zero_lit(&cond.right) {
                    &cond.left
                } else if is_zero_lit(&cond.left) {
                    &cond.right
                } else {
                    return None;
                };
                (DivArm::Else, d)
            }
            BinOp::Ne(_) => {
                let d = if is_zero_lit(&cond.right) {
                    &cond.left
                } else if is_zero_lit(&cond.left) {
                    &cond.right
                } else {
                    return None;
                };
                (DivArm::Then, d)
            }
            BinOp::Gt(_) => {
                if !is_zero_lit(&cond.right) {
                    return None;
                }
                (DivArm::Then, &cond.left)
            }
            BinOp::Lt(_) => {
                if !is_zero_lit(&cond.left) {
                    return None;
                }
                (DivArm::Then, &cond.right)
            }
            _ => return None,
        };
        // Divisor must be a plain path/field — evaluated twice (cond + div),
        // so method calls/indexing stay silent (matches real clippy's refusal
        // of `q.len()` divisors), and a bare path that is PROVABLY signed is
        // refused (real clippy never fires there).
        match peel_parens(divisor) {
            Expr::Path(_) | Expr::Field(_) => {}
            _ => return None,
        }
        if let Some(id) = bare_ident(divisor) {
            if self
                .signed_idents
                .last()
                .is_some_and(|set| set.contains(&id))
            {
                return None;
            }
        }
        let div_expr = Self::arm_expr(e, div_arm)?;
        let val_expr = Self::arm_expr(e, if div_arm == DivArm::Then { DivArm::Else } else { DivArm::Then })?;
        let Expr::Binary(div) = peel_parens(div_expr) else {
            return None;
        };
        if !matches!(div.op, BinOp::Div(_)) {
            return None; // `%` guards are not this lint
        }
        // The division's divisor must be TEXTUALLY the condition's divisor.
        if src(self.source, &div.right)? != src(self.source, divisor)? {
            return None;
        }
        // Whole-if replacement — any interior comment would be displaced.
        if drops_interior_comment(self.source, byte_range(e), &[]) {
            return None;
        }
        let num_text = src(self.source, &div.left)?;
        let divisor_text = src(self.source, &div.right)?;
        let val_text = src(self.source, val_expr)?;
        let recv = if is_postfix_safe(&div.left) {
            num_text
        } else {
            format!("({num_text})")
        };
        Some(format!("{recv}.checked_div({divisor_text}).unwrap_or({val_text})"))
    }

    /// The single expression of one if arm. `then_branch` is always a block;
    /// `else_branch` is `Some` (an `else if` chain fails the block-shape
    /// extraction and refuses).
    fn arm_expr(e: &ExprIf, arm: DivArm) -> Option<&Expr> {
        match arm {
            DivArm::Then => block_single_expr(&e.then_branch),
            DivArm::Else => {
                // syn 2's else_branch carries the `else` token: `(Else, Box<Expr>)`.
                let else_e = e.else_branch.as_ref()?.1.as_ref();
                match else_e {
                    Expr::Block(b) => block_single_expr(&b.block),
                    _ => None,
                }
            }
        }
    }
}

fn block_single_expr(block: &syn::Block) -> Option<&Expr> {
    if block.stmts.len() != 1 {
        return None;
    }
    match &block.stmts[0] {
        Stmt::Expr(e, _) => Some(e),
        _ => None,
    }
}

fn peel_parens(e: &Expr) -> &Expr {
    let mut cur = e;
    while let Expr::Paren(ExprParen { expr, .. }) = cur {
        cur = expr;
    }
    cur
}

fn bare_ident(e: &Expr) -> Option<String> {
    let Expr::Path(ExprPath { path, .. }) = peel_parens(e) else {
        return None;
    };
    path.get_ident().map(ToString::to_string)
}

fn is_zero_lit(e: &Expr) -> bool {
    int_lit(e).is_some_and(|(digits, _)| digits == "0")
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn eq_zero_form_fires() {
        let input = "fn f(n: u64, total: u64) -> u64 { if n == 0 { 0 } else { total / n } }";
        let out = propose_fix("manual_checked_ops", input).expect("matched");
        assert!(out.contains("total.checked_div(n).unwrap_or(0)"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn zero_on_left_fires() {
        let input = "fn f(n: usize, total: usize) -> usize { if 0 == n { 7 } else { total / n } }";
        let out = propose_fix("manual_checked_ops", input).expect("matched");
        assert!(out.contains("total.checked_div(n).unwrap_or(7)"), "{out}");
    }

    #[test]
    fn inverted_form_fires() {
        let input = "fn f(n: usize, total: usize) -> usize { if n != 0 { total / n } else { 9 } }";
        let out = propose_fix("manual_checked_ops", input).expect("matched");
        assert!(out.contains("total.checked_div(n).unwrap_or(9)"), "{out}");
    }

    #[test]
    fn gt_zero_form_fires() {
        let input = "fn f(n: u32, v: u32) -> u32 { if n > 0 { v / n } else { 1 } }";
        let out = propose_fix("manual_checked_ops", input).expect("matched");
        assert!(out.contains("v.checked_div(n).unwrap_or(1)"), "{out}");
    }

    #[test]
    fn field_divisor_fires() {
        let input = "struct C { denom: usize }\nfn f(c: &C, total: usize) -> usize { if c.denom == 0 { 3 } else { total / c.denom } }";
        let out = propose_fix("manual_checked_ops", input).expect("matched");
        assert!(out.contains("total.checked_div(c.denom).unwrap_or(3)"), "{out}");
    }

    #[test]
    fn stmt_position_fires() {
        let input = "fn f(n: usize, out: &mut usize) { *out = if n == 0 { 0 } else { 42 / n }; }";
        let out = propose_fix("manual_checked_ops", input).expect("matched");
        assert!(out.contains("42.checked_div(n).unwrap_or(0);"), "{out}");
    }

    #[test]
    fn complex_numerator_parenthesizes() {
        let input = "fn f(n: usize, a: usize, b: usize) -> usize { if n == 0 { 0 } else { (a + b) / n } }";
        let out = propose_fix("manual_checked_ops", input).expect("matched");
        assert!(out.contains("(a + b).checked_div(n).unwrap_or(0)"), "{out}");
    }

    #[test]
    fn signed_param_refused() {
        let input = "fn f(n: i64, total: i64) -> i64 { if n == 0 { 0 } else { total / n } }";
        let out = propose_fix("manual_checked_ops", input);
        assert!(out.is_none(), "provably-signed divisor must refuse: {out:?}");
    }

    #[test]
    fn method_call_divisor_refused() {
        let input = "fn f(q: &Vec<u8>, total: usize) -> usize { if q.len() == 0 { 0 } else { total / q.len() } }";
        let out = propose_fix("manual_checked_ops", input);
        assert!(out.is_none(), "method divisor must refuse: {out:?}");
    }

    #[test]
    fn rem_guard_not_fired() {
        let input = "fn f(n: usize, total: usize) -> usize { if n == 0 { 0 } else { total % n } }";
        let out = propose_fix("manual_checked_ops", input);
        assert!(out.is_none(), "% is not this lint: {out:?}");
    }

    #[test]
    fn mismatched_divisor_refused() {
        let input = "fn f(n: usize, m: usize, total: usize) -> usize { if n == 0 { 0 } else { total / m } }";
        let out = propose_fix("manual_checked_ops", input);
        assert!(out.is_none(), "cond/divisor mismatch must refuse: {out:?}");
    }

    #[test]
    fn multi_stmt_arm_refused() {
        let input = "fn f(n: usize, total: usize) -> usize { if n == 0 { log(); 0 } else { total / n } }";
        let out = propose_fix("manual_checked_ops", input);
        assert!(out.is_none(), "side-effecting guard arm must refuse: {out:?}");
    }

    #[test]
    fn interior_comment_refused() {
        let input = "fn f(n: usize, total: usize) -> usize { if n == 0 { 0 } else { total /* c */ / n } }";
        let out = propose_fix("manual_checked_ops", input);
        assert!(out.is_none(), "interior comment must refuse: {out:?}");
    }
}

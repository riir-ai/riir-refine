//! `clippy::bool_comparison` — `x == true` → `x`, `x != true` → `!x`,
//! `x == false` → `!x`, `x != false` → `x` (Batch 162 / Plan 153 — the
//! frontier-lead mining pack VIII; the family fixture verified to fire
//! exactly this lint under all+pedantic+nursery with a clean seed,
//! 2026-09-18 (`scripts/plan152_frontier_probe.py`)).
//!
//! ## Why the rewrite is boolean algebra (the FIXABLE bar)
//!
//! `x == true` IS `x` and `x != true` IS `!x` on a bool operand — the
//! equality/inequality with a literal is the identity/negation law of the
//! two-element Boolean algebra, and Rust's `bool` is exactly that
//! (`PartialEq` + `not`; there is no third value, no NaN class). `false` twins
//! mirror: `x == false` is `!x`, `x != false` is `x`. The rewritten
//! expression has the same `bool` type in every context the original
//! appeared in (if/while conditions, tails, args, `&&`/`||` chains — the
//! emit replaces the whole binary node).
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - The NON-LITERAL operand must be a BARE IDENT proven `bool` by the
//!   enclosing fn's signature (a param annotated `bool`) — the same
//!   construction-fact provenance the Batch-158/159/160 scalar derivations
//!   use (`helpers::scalar_param_heads_over` already admits `bool`; the
//!   never-rebound guard makes the annotation a construction fact).
//!   Everything else — literals, calls, fields, indexed places, casts —
//!   refuses. A LITERAL-only comparison (`true == true`) refuses: no
//!   proven ident.
//! - The literal side must be exactly `true`/`false` (no parens unwrap on
//!   the literal — a `(true)` spelling rides the snippet check only via the
//!   unwrapped literal; the emit uses the canonical spelling anyway).
//! - Operand order: literal-on-RHS (the canonical spelling) and
//!   literal-on-LHS (`true == x`) both rewrite to the same canonical form —
//!   `==` is commutative and `!=` is too, so both orders carry the same
//!   identity (unlike the range rule's forms A/B, which are two DIFFERENT
//!   lattices). The non-literal side keeps its byte-identical text.
//! - Comments inside the replaced span refuse; an operand that is NOT the
//!   proven ident and NOT the bool literal refuses.
//! - Macro-invocation operands refuse by construction (the literal check is
//!   `Expr::Lit`).

use std::collections::BTreeMap;

use syn::visit::Visit;
use syn::{BinOp, Expr, ExprBinary, File, ImplItemFn, ItemFn, Lit};

use super::helpers::{byte_range, drops_interior_comment, scalar_param_heads_over};
use super::{edit, SourceEdit};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        proven: BTreeMap::new(),
        saved: BTreeMap::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Signature-proven bindings of the enclosing fn (type text).
    proven: BTreeMap<String, String>,
    /// The set to restore on fn exit (one-deep save).
    saved: BTreeMap<String, String>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = bool_param_heads_over(f.sig.inputs.iter(), &f.block);
        self.saved = saved;
        syn::visit::visit_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = bool_param_heads_over(f.sig.inputs.iter(), &f.block);
        self.saved = saved;
        syn::visit::visit_impl_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_expr_binary(&mut self, b: &'ast ExprBinary) {
        syn::visit::visit_expr_binary(self, b);
        if let Some(replacement) = self.try_rewrite(b) {
            self.edits.push(edit(b, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_rewrite(&self, b: &ExprBinary) -> Option<String> {
        let eq = match b.op {
            BinOp::Eq(_) => true,
            BinOp::Ne(_) => false,
            _ => return None,
        };
        // Literal side can be either operand; the other must be the proven
        // bare ident.
        let (lhs, rhs) = (&b.left, &b.right);
        if let Some((x_expr, lit_val)) = split_operand(lhs, rhs) {
            self.emit(b, x_expr, lit_val, eq)
        } else if let Some((x_expr, lit_val)) = split_operand(rhs, lhs) {
            self.emit(b, x_expr, lit_val, eq)
        } else {
            None
        }
    }

    fn emit(&self, b: &ExprBinary, x_expr: &Expr, lit_val: bool, eq: bool) -> Option<String> {
        let Expr::Path(_) = x_expr else { return None };
        let x_range = byte_range(x_expr);
        let x = self.source.get(x_range.clone())?.to_string();
        let ident = x.clone();
        if !self.proven.contains_key(ident.as_str()) {
            return None;
        }
        if drops_interior_comment(self.source, byte_range(b), &[x_range]) {
            return None;
        }
        // (eq, lit) → bool algebra identity: == true / != false → x;
        // != true / == false → !x.
        let keep = eq == lit_val;
        if keep {
            Some(x)
        } else {
            Some(format!("!{x}"))
        }
    }
}

/// Split the pair into (proven-ident candidate, literal value): the second
/// operand must be a bool literal, the first a plain path. Either side
/// being anything else refuses.
fn split_operand<'a>(
    x_expr: &'a Expr,
    lit_expr: &'a Expr,
) -> Option<(&'a Expr, bool)> {
    let Expr::Lit(l) = lit_expr else { return None };
    let lit_val = match &l.lit {
        Lit::Bool(b) => b.value,
        _ => return None,
    };
    // Only a bare path (single segment, no arguments) is the ident shape.
    let Expr::Path(p) = x_expr else { return None };
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    if !p.path.segments[0].arguments.is_none() {
        return None;
    }
    Some((x_expr, lit_val))
}

/// Signature-proven `bool` bindings of `f` (the boolean slice of
/// [`scalar_param_heads_over`]: the derivation already admits `bool`
/// annotations, and the never-rebound guard is the soundness leg).
fn bool_param_heads_over<'i>(
    inputs: impl Iterator<Item = &'i syn::FnArg>,
    block: &syn::Block,
) -> BTreeMap<String, String> {
    scalar_param_heads_over(inputs, block)
        .into_iter()
        .filter(|(_, ty)| ty == "bool")
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            "bool_comparison",
            "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x }",
        )
        .is_none());
    }

    #[test]
    fn eq_true_fires() {
        let out = propose_fix(
            "bool_comparison",
            "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x == true }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(x); x }"), "not simplified: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn ne_true_fires_to_negation() {
        let out = propose_fix(
            "bool_comparison",
            "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x != true }",
        )
        .expect("matched");
        assert!(out.contains("!x"), "not negated: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn eq_false_fires_to_negation() {
        let out = propose_fix(
            "bool_comparison",
            "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x == false }",
        )
        .expect("matched");
        assert!(out.contains("!x"), "not negated: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn ne_false_fires() {
        let out = propose_fix(
            "bool_comparison",
            "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x != false }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(x); x }"), "not simplified: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn literal_on_lhs_fires_same_way() {
        // `==` is commutative, so the mirror order carries the SAME identity:
        // `true == x` IS `x`.
        let out = propose_fix(
            "bool_comparison",
            "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); true == x }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(x); x }"), "mirror not simplified: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn unproven_operand_refuses() {
        // A non-bool operand (the literal check is syntactic; the proven
        // map carries the type). `x` here is u32 — refuse.
        assert!(propose_fix(
            "bool_comparison",
            "fn f(x: u32, hits: &mut Vec<bool>) -> bool { hits.push(false); x == true }",
        )
        .is_none());
    }

    #[test]
    fn call_operand_refuses() {
        // A call operand is not a bare proven ident — refuse even beside a
        // literal.
        assert!(propose_fix(
            "bool_comparison",
            "fn f(hits: &mut Vec<bool>) -> bool { hits.push(false); is_on(hits) == true } fn is_on(_h: &mut Vec<bool>) -> bool { false }",
        )
        .is_none());
    }

    #[test]
    fn nested_in_condition_fires() {
        let out = propose_fix(
            "bool_comparison",
            "fn pick(x: bool, hits: &mut Vec<bool>) -> u32 { hits.push(x); if x == true { 1 } else { 0 } }",
        )
        .expect("matched");
        assert!(out.contains("if x { 1 }"), "condition not simplified: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            "bool_comparison",
            "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x /* why */ == true }",
        )
        .is_none());
    }

    #[test]
    fn impl_method_param_proven() {
        let out = propose_fix(
            "bool_comparison",
            "struct S; impl S { fn flag(&self, x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x == true } }",
        )
        .expect("matched");
        assert!(out.contains("x } }"), "not simplified: {out}");
        syn::parse_file(&out).expect("re-parses");
    }
}

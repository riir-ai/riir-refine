//! `clippy::needless_range_loop` — `for i in 0..v.len() { body_using(v[i]) }`
//! → `for __elem in &v { body_with_elem }`.
//!
//! Span-preserving (T6.4): three edit kinds — the loop pat (`i` → the fresh
//! binding), the iterable (`0..v.len()` → the collection, by-reference form
//! chosen by param resolution), and each exact `v[i]` index expression (→
//! the fresh binding). All replacement fragments are ORIGINAL source text.
//!
//! ## Detection (exact AST analysis, replacing the token heuristic)
//!
//! The mutation-based version counted identifier occurrences in the body's
//! token stream — substring-based, so `minimum` counted as a use of `i`
//! (false negatives) and nested `v[v[i]]` slipped through as a false
//! POSITIVE (it rewrote to `v[__elem]`, indexing by an element). It also
//! emitted `&v` unconditionally — for a `v: &Vec<_>` param that iterates
//! `&&Vec`, which is not an iterator (the fixture compiled only because
//! nothing ever compiled it). The exact visitor tracks:
//!
//! - `exact` — `Expr::Index` nodes whose base is the collection path AND
//!   whose index is the bare loop ident (the substitution slots);
//! - `total` — every bare `Expr::Path` that IS the loop ident in the body
//!   (the default traversal counts the ident inside every index too — an
//!   index into a DIFFERENT collection, `w[i]`, therefore makes
//!   `total > exact` and blocks the rewrite);
//! - `complex` — any `Expr::Index` whose index mentions the ident without
//!   being the bare ident itself (`v[i + 1]`, `v[v[i]]`);
//! - `blocked` — an exact slot is written through (`v[i] = x`, `v[i] += x`)
//!   or has its address taken (`&v[i]`, `&mut v[i]`): the element binding
//!   is a shared reference and cannot stand in for a place expression.
//!
//! The rewrite fires iff `exact >= 1 && total == exact && !complex && !blocked`.
//!
//! ## Iterable form (param resolution)
//!
//! `--fix` has no type checker, so the collection ident is resolved against
//! the enclosing fn's parameters (the only source of truth available):
//! a `&`-typed param iterates as `v`, an owned param as `&v`. Anything the
//! resolver cannot pin (locals, fields, non-ident paths) skips the rewrite.

use super::{edit, for_each_fn, src, SourceEdit};
use syn::visit::Visit;
use syn::{BinOp, Expr, ExprForLoop, ExprLit, ExprRange, File, Lit, Signature};

/// Fresh binding introduced by the rewrite. The loop is skipped entirely if
/// the body already mentions any identifier with this prefix.
const FRESH_PREFIX: &str = "__elem";

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    for_each_fn(file, |sig, block| {
        let mut v = Collector { source, sig, edits: &mut edits };
        v.visit_block(block);
    });
    edits
}

struct Collector<'a, 'b> {
    source: &'a str,
    sig: &'b Signature,
    edits: &'b mut Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_, '_> {
    fn visit_expr_for_loop(&mut self, fl: &'ast ExprForLoop) {
        syn::visit::visit_expr_for_loop(self, fl);
        if let Some(mut loop_edits) = self.try_rewrite_for(fl) {
            self.edits.append(&mut loop_edits);
        }
    }
}

impl Collector<'_, '_> {
    fn try_rewrite_for(&self, fl: &ExprForLoop) -> Option<Vec<SourceEdit>> {
        if !fl.attrs.is_empty() {
            return None; // attrs would be lost by the header splice
        }
        let Expr::Range(range) = &*fl.expr else {
            return None;
        };
        check_range_start_zero(range)?;
        let collection_path = check_range_end_len(range)?;

        // The loop ident: a bare (non-ref, no subpat) binding.
        let syn::Pat::Ident(pi) = &*fl.pat else {
            return None;
        };
        if pi.by_ref.is_some() || pi.subpat.is_some() {
            return None;
        }
        let idx_ident = &pi.ident;

        // Exact usage analysis over the BODY only (the header's `i` is not a use).
        let usage = analyze_index_usage(&fl.body, collection_path, idx_ident);
        if !usage.ok() {
            return None;
        }

        // Fresh binding must not collide with anything the body mentions.
        if body_mentions_prefix(&fl.body) {
            return None;
        }

        // Iterable form: resolve the collection against the fn params —
        // a `&`-typed param iterates bare, an owned param by reference.
        // Unresolvable collections are skipped (no type checker to ask).
        let coll_text = src(self.source, collection_path)?;
        let iter_text = match self.param_is_reference(collection_path) {
            Some(true) => coll_text,
            Some(false) => format!("&{coll_text}"),
            None => return None,
        };

        let mut edits = vec![
            edit(&fl.pat, FRESH_PREFIX.to_string()),
            edit(&fl.expr, iter_text),
        ];
        for whole in usage.exact {
            edits.push(edit(whole, FRESH_PREFIX.to_string()));
        }
        Some(edits)
    }

    /// Is the single-ident path a fn parameter of reference type?
    /// `None` = not a resolvable parameter (skip the rewrite).
    fn param_is_reference(&self, path: &syn::ExprPath) -> Option<bool> {
        let ident = path.path.get_ident()?;
        for arg in &self.sig.inputs {
            let syn::FnArg::Typed(pat) = arg else { continue };
            let syn::Pat::Ident(pi) = &*pat.pat else { continue };
            if pi.ident != *ident {
                continue;
            }
            return Some(matches!(&*pat.ty, syn::Type::Reference(_) | syn::Type::Paren(_)));
        }
        None
    }
}

fn check_range_start_zero(range: &ExprRange) -> Option<()> {
    let start = range.start.as_deref()?;
    if matches!(start, Expr::Lit(ExprLit { lit: Lit::Int(li), .. }) if li.base10_digits() == "0") {
        Some(())
    } else {
        None
    }
}

fn check_range_end_len(range: &ExprRange) -> Option<&syn::ExprPath> {
    let end = range.end.as_deref()?;
    let end_call = match end {
        Expr::MethodCall(mc) if mc.method == "len" && mc.args.is_empty() => &*mc.receiver,
        _ => return None,
    };
    if let Expr::Path(p) = end_call {
        Some(p)
    } else {
        None
    }
}

struct IndexUsage<'a> {
    /// Whole `collection[idx]` nodes — the substitution slots.
    exact: Vec<&'a syn::ExprIndex>,
    /// Every bare `Expr::Path` that IS the loop ident in the body.
    total: usize,
    /// An index expression mentions the ident without being the bare ident.
    complex: bool,
    /// An exact slot is written through or has its address taken.
    blocked: bool,
}

impl IndexUsage<'_> {
    fn ok(&self) -> bool {
        !self.exact.is_empty() && self.total == self.exact.len() && !self.complex && !self.blocked
    }
}

fn analyze_index_usage<'a>(
    body: &'a syn::Block,
    collection: &syn::ExprPath,
    idx: &syn::Ident,
) -> IndexUsage<'a> {
    struct Scan<'a, 'b> {
        collection: &'b syn::ExprPath,
        idx: &'b syn::Ident,
        usage: IndexUsage<'a>,
    }
    impl<'ast> Visit<'ast> for Scan<'ast, '_> {
        fn visit_expr_index(&mut self, e: &'ast syn::ExprIndex) {
            syn::visit::visit_expr_index(self, e);
            let idx_is_bare = matches!(&*e.index, Expr::Path(p) if p.path.is_ident(self.idx));
            if !idx_is_bare {
                // `v[i + 1]`, `v[v[i]]` — index mentions the ident but isn't
                // the bare ident: substitution cannot cover it.
                if expr_mentions_ident(&e.index, self.idx) {
                    self.usage.complex = true;
                }
                return;
            }
            // `w[i]` (other base) is NOT pushed as an exact slot, but the
            // traversal already counted its inner `i` path toward `total` —
            // so `total > exact.len()` blocks the rewrite, as it must.
            if matches!(&*e.expr, Expr::Path(p) if paths_equal(p, self.collection)) {
                self.usage.exact.push(e);
            }
        }

        fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
            if p.path.is_ident(self.idx) {
                self.usage.total += 1;
            }
            syn::visit::visit_expr_path(self, p);
        }

        // `v[i] = x` — the slot is a place expression; the element binding
        // is a shared reference and cannot be assigned through.
        fn visit_expr_assign(&mut self, e: &'ast syn::ExprAssign) {
            syn::visit::visit_expr_assign(self, e);
            if self.is_exact_slot(&e.left) {
                self.usage.blocked = true;
            }
        }

        // `v[i] += x` etc. — compound assigns are place writes too.
        fn visit_expr_binary(&mut self, e: &'ast syn::ExprBinary) {
            syn::visit::visit_expr_binary(self, e);
            if is_assign_op(&e.op) && self.is_exact_slot(&e.left) {
                self.usage.blocked = true;
            }
        }

        // `&v[i]` / `&mut v[i]` — address-of through the slot changes the
        // referent (`&T` → `&&T`).
        fn visit_expr_reference(&mut self, e: &'ast syn::ExprReference) {
            syn::visit::visit_expr_reference(self, e);
            if self.is_exact_slot(&e.expr) {
                self.usage.blocked = true;
            }
        }
    }
    impl Scan<'_, '_> {
        fn is_exact_slot(&self, e: &Expr) -> bool {
            matches!(e, Expr::Index(ix)
                if matches!(&*ix.index, Expr::Path(p) if p.path.is_ident(self.idx))
                    && matches!(&*ix.expr, Expr::Path(p) if paths_equal(p, self.collection)))
        }
    }
    let mut scan = Scan {
        collection,
        idx,
        usage: IndexUsage { exact: Vec::new(), total: 0, complex: false, blocked: false },
    };
    scan.visit_block(body);
    scan.usage
}

fn is_assign_op(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::AddAssign(_)
            | BinOp::SubAssign(_)
            | BinOp::MulAssign(_)
            | BinOp::DivAssign(_)
            | BinOp::RemAssign(_)
            | BinOp::BitXorAssign(_)
            | BinOp::BitAndAssign(_)
            | BinOp::BitOrAssign(_)
            | BinOp::ShlAssign(_)
            | BinOp::ShrAssign(_)
    )
}

/// Does the expression mention the ident as a bare path anywhere?
fn expr_mentions_ident(e: &Expr, idx: &syn::Ident) -> bool {
    struct M<'b> {
        idx: &'b syn::Ident,
        found: bool,
    }
    impl<'ast> Visit<'ast> for M<'_> {
        fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
            if p.path.is_ident(self.idx) {
                self.found = true;
            }
            syn::visit::visit_expr_path(self, p);
        }
    }
    let mut m = M { idx, found: false };
    m.visit_expr(e);
    m.found
}

/// Does the body mention any identifier starting with the fresh prefix?
fn body_mentions_prefix(body: &syn::Block) -> bool {
    struct P<'b> {
        prefix: &'b str,
        found: bool,
    }
    impl<'ast> Visit<'ast> for P<'_> {
        fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
            if p.path.get_ident().is_some_and(|id| id.to_string().starts_with(self.prefix)) {
                self.found = true;
            }
            syn::visit::visit_expr_path(self, p);
        }
    }
    let mut p = P { prefix: FRESH_PREFIX, found: false };
    p.visit_block(body);
    p.found
}

fn paths_equal(a: &syn::ExprPath, b: &syn::ExprPath) -> bool {
    a.path.segments.len() == b.path.segments.len()
        && a.path
            .segments
            .iter()
            .zip(&b.path.segments)
            .all(|(sa, sb)| sa.ident == sb.ident)
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_range_loop_ref_param_bare_iter() {
        // `v: &Vec<_>` — the collection is already a reference; `&v` would
        // iterate `&&Vec` (not an iterator). The old proposer had this bug.
        let input = "fn double_all(v: &Vec<i32>) -> Vec<i32> {\n    let mut out = Vec::new();\n    for i in 0..v.len() { out.push(v[i] * 2); }\n    out\n}";
        let out = propose_fix("needless_range_loop", input).expect("matched");
        assert!(
            out.contains("for __elem in v { out.push(__elem * 2); }"),
            "ref-param iter loop not produced: {out}"
        );
        assert!(!out.contains("0.."), "range not removed: {out}");
    }

    #[test]
    fn converts_range_loop_owned_param_ref_iter() {
        let input = "fn sum(v: Vec<i32>) -> i32 { let mut s = 0; for i in 0..v.len() { s += v[i]; } s }";
        let out = propose_fix("needless_range_loop", input).expect("matched");
        assert!(out.contains("for __elem in &v { s += __elem; }"), "{out}");
    }

    #[test]
    fn slice_param_bare_iter() {
        let input = "fn sum(v: &[i32]) -> i32 { let mut s = 0; for i in 0..v.len() { s += v[i]; } s }";
        let out = propose_fix("needless_range_loop", input).expect("matched");
        assert!(out.contains("for __elem in v {"), "{out}");
    }

    #[test]
    fn leaves_non_range_loop_unchanged() {
        let input = "fn go(v: Vec<i32>) { for x in v { println!(\"{}\", x); } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "non-range loop touched: {out:?}");
    }

    #[test]
    fn leaves_loop_with_arithmetic_index() {
        let input = "fn go(v: Vec<i32>) { for i in 0..v.len() { println!(\"{}\", v[i] + i); } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "arithmetic-index loop touched: {out:?}");
    }

    /// `v[i + 1]` — complex index mentioning the ident: rejected exactly.
    #[test]
    fn leaves_loop_with_offset_index() {
        let input = "fn go(v: Vec<i32>) { for i in 0..v.len() { println!(\"{}\", v[i + 1]); } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "offset-index loop touched: {out:?}");
    }

    /// `v[v[i]]` — the outer index is not the bare ident: the old token
    /// heuristic false-positived here (it rewrote to `v[__elem]`).
    #[test]
    fn leaves_loop_with_nested_index() {
        let input = "fn go(v: Vec<i32>) { for i in 0..v.len() { println!(\"{}\", v[v[i]]); } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "nested-index loop touched: {out:?}");
    }

    /// `w[i]` — the ident indexes a DIFFERENT collection: cannot rewrite.
    #[test]
    fn leaves_loop_indexing_other_collection() {
        let input =
            "fn go(v: Vec<i32>, w: Vec<i32>) { for i in 0..v.len() { println!(\"{}\", w[i]); } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "other-collection index touched: {out:?}");
    }

    /// `v[i] = 5` — a place write: the element binding is a shared ref.
    #[test]
    fn leaves_loop_with_slot_write() {
        let input = "fn go(v: Vec<i32>) { for i in 0..v.len() { v[i] = 5; } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "slot-writing loop touched: {out:?}");
    }

    /// `v[i] += 1` — compound-assign place write.
    #[test]
    fn leaves_loop_with_slot_compound_write() {
        let input = "fn go(v: Vec<i32>) -> i32 { let mut s = 0; for i in 0..v.len() { v[i] += 1; s += v[i]; } s }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "compound-write loop touched: {out:?}");
    }

    /// `&v[i]` — address-of through the slot (`&T` → `&&T` under rewrite).
    #[test]
    fn leaves_loop_with_slot_ref() {
        let input = "fn go(v: Vec<i32>) -> i32 { let mut s = 0; for i in 0..v.len() { s += *(&v[i]); } s }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "slot-ref loop touched: {out:?}");
    }

    /// A local (non-param) collection cannot be resolved to ref-or-owned —
    /// skip rather than guess the iterable form.
    #[test]
    fn leaves_loop_over_local_collection() {
        let input = "fn go() -> i32 { let v = vec![1, 2, 3]; let mut s = 0; for i in 0..v.len() { s += v[i]; } s }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "unresolvable local collection must skip: {out:?}");
    }

    /// `minimum` contains but is not `i` — the old token heuristic treated
    /// it as an idx use (false negative); the exact visitor does not.
    #[test]
    fn substring_identifiers_do_not_count_as_uses() {
        let input = "fn go(v: &[i32], minimum: i32) -> i32 { let mut s = 0; for i in 0..v.len() { if v[i] > minimum { s += v[i]; } } s }";
        let out = propose_fix("needless_range_loop", input).expect("matched");
        assert!(out.contains("for __elem in v {"), "{out}");
        assert!(out.contains("if __elem > minimum"), "{out}");
        assert!(out.contains("s += __elem"), "{out}");
    }

    #[test]
    fn fresh_prefix_collision_skips() {
        let input = "fn go(v: &[i32]) -> i32 { let mut s = 0; let __elem = 1; for i in 0..v.len() { s += v[i] + __elem; } s }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "fresh-name collision must skip: {out:?}");
    }

    #[test]
    fn unused_idx_in_range_loop_unchanged() {
        // The index never appears in the body — rewriting to `for _ in &v`
        // is a different lint (unused loop variable is not ours to rewrite).
        let input = "fn go(v: Vec<i32>) { for i in 0..v.len() { println!(\"tick\"); } }";
        let out = propose_fix("needless_range_loop", input);
        assert!(out.is_none(), "unused-idx loop touched: {out:?}");
    }

    #[test]
    fn impl_method_param_resolves() {
        let input = "struct S;\nimpl S {\n    fn total(&self, v: &Vec<i32>) -> i32 { let mut s = 0; for i in 0..v.len() { s += v[i]; } s }\n}";
        let out = propose_fix("needless_range_loop", input).expect("matched");
        assert!(out.contains("for __elem in v { s += __elem; }"), "{out}");
    }
}

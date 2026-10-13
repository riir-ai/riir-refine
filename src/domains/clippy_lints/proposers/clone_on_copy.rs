//! `clippy::clone_on_copy` — `x.clone()` → `x` on a proven-Copy receiver
//! (Batch 167 / Plan 158 — the coverage-report re-arm pack XI; the family
//! fixture verified to fire exactly this lint under all+pedantic+nursery
//! with a clean fixed form, 2026-09-21,
//! `scripts/plan158_frontier_probe.py`).
//!
//! ## Batch 230 rider (Plan 232): the UFCS path-call forms
//!
//! clippy 1.99 (#16972) widened the lint to UFCS calls — `Clone::clone(&x)`,
//! `u32::clone(&x)`, `<u32 as Clone>::clone(&x)` — all probe-verified firing
//! 2026-10-13 (`scripts/plan232_batch230_probe/`, isolated +1.99.0
//! toolchain). The rider admits the shape the doc previously refused: a
//! path-call whose SINGLE argument is `&bare_ident` with the ident a
//! signature-proven Copy scalar. The fix is `ident` — `Clone for T` on a
//! Copy `T` is `*x` by the std contract (the same FIXABLE bar as the method
//! form). By-ref argument spellings (`Clone::clone(s)` for `s: &str`),
//! `&mut`, and multi-arg forms refuse — narrower than clippy, by design.
//!
//! ## Why the rewrite is construction-identity (the FIXABLE bar)
//!
//! On a `Copy` type `T`, `x.clone()` and `x` have the same type, the same
//! value, and the same move/copy semantics at every use site (`Clone` for a
//! `Copy` type is REQUIRED to be `*x` by the std contract — "should not be
//! overridden"; a type that breaks that contract is unsound by its own
//! documentation). Dropping the call therefore rewrites nothing but the
//! syntax — and skips an autoderef-then-clone hop.
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - The receiver must be a BARE IDENT proven by the enclosing fn's
//!   signature via `scalar_param_heads_over`: every admitted scalar is
//!   `Copy` (all numeric primitives, `bool`, `char`) and the shared-reference
//!   spellings `&str` / `&String` are `Copy` too (`&T` is `Copy` for every
//!   `T`). `String` is NOT `Copy` and refuses by construction — the probe
//!   showed a non-Copy clone fires `redundant_clone` (a DIFFERENT,
//!   value-flow lint), never ours.
//! - Arrays / tuples / `&mut T` receivers refuse: they are not in the
//!   proven map (v1 stays on the scalar derivation; a Copy-ANALYSIS
//!   collector is the later rider).
//! - The method must be exactly `clone` with ZERO arguments and no
//!   turbofish; path-call spellings (`T::clone(&x)`) refuse. A receiver
//!   that is a method chain, field, index, or call refuses (not a bare
//!   proven ident).
//! - **Batch 230**: the path-call refusal above is LIFTED for the UFCS
//!   family (see the rider section) — `Clone::clone(&x)`-shaped calls on
//!   proven-Copy scalars now rewrite to `x`. Everything else about the
//!   original refusal stands.
//! - Distinctness: `needless_borrow` owns expression-POSITION borrows;
//!   `redundant_clone` owns value-flow clones that are never used; this
//!   rule owns only the proven-Copy receiver's semantic no-op. Comments
//!   inside the replaced span refuse.

use std::collections::BTreeMap;

use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, File, ImplItemFn, ItemFn};

use super::helpers::{byte_range, drops_interior_comment, scalar_param_heads_over};
use super::{edit, SourceEdit};

/// The proven types whose `.clone()` is a semantic no-op: every scalar
/// primitive plus the shared-reference spellings the scalar derivation
/// admits. `String` is deliberately absent — it is not `Copy`.
const COPY_TYPES: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128",
    "isize", "f32", "f64", "bool", "char", "&str", "&String",
];

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
        self.proven = scalar_param_heads_over(f.sig.inputs.iter(), &f.block);
        self.saved = saved;
        syn::visit::visit_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = scalar_param_heads_over(f.sig.inputs.iter(), &f.block);
        self.saved = saved;
        syn::visit::visit_impl_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        if let Some(replacement) = self.try_rewrite(m) {
            self.edits.push(edit(m, replacement));
        }
    }

    fn visit_expr_call(&mut self, c: &'ast ExprCall) {
        // The Batch-230 UFCS rider: `Clone::clone(&x)`-family path calls.
        // Descend first (default traversal), then attempt the rewrite so a
        // nested non-matching call inside the argument still gets its own
        // visit.
        syn::visit::visit_expr_call(self, c);
        if let Some(replacement) = self.try_rewrite_ufcs(c) {
            self.edits.push(edit(c, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_rewrite(&self, m: &ExprMethodCall) -> Option<String> {
        if m.method != "clone" || !m.args.is_empty() {
            return None;
        }
        if m.turbofish.is_some() {
            return None;
        }
        let Expr::Path(p) = &*m.receiver else { return None };
        if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
            return None;
        }
        if !p.path.segments[0].arguments.is_none() {
            return None;
        }
        let ident = p.path.segments[0].ident.to_string();
        let ty = self.proven.get(ident.as_str())?;
        if !COPY_TYPES.contains(&ty.as_str()) {
            return None;
        }
        if drops_interior_comment(self.source, byte_range(m), &[byte_range(&*m.receiver)]) {
            return None;
        }
        Some(ident)
    }

    /// The Batch-230 UFCS arm: `Clone::clone(&x)` / `Ty::clone(&x)` /
    /// `<Ty as Clone>::clone(&x)` / `std::clone::Clone::clone(&x)` where
    /// `x` is a signature-proven Copy scalar BY VALUE. The fix is `x`.
    fn try_rewrite_ufcs(&self, c: &ExprCall) -> Option<String> {
        let Expr::Path(p) = &*c.func else { return None };
        if p.path.leading_colon.is_some() || c.args.len() != 1 {
            return None;
        }
        let segs: Vec<String> = p
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        // Every segment must be argument-free (a turbofish on any segment
        // refuses; qself already excluded by the Expr::Path shape).
        if p.path.segments.iter().any(|s| !s.arguments.is_none()) {
            return None;
        }
        // The single argument must be `&bare_ident`.
        let arg = c.args.first()?;
        let Expr::Reference(r) = arg else { return None };
        if r.mutability.is_some() {
            return None;
        }
        let Expr::Path(ap) = &*r.expr else { return None };
        if ap.qself.is_some() || ap.path.leading_colon.is_some() || ap.path.segments.len() != 1 {
            return None;
        }
        if !matches!(ap.path.segments[0].arguments, syn::PathArguments::None) {
            return None;
        }
        let ident = ap.path.segments[0].ident.to_string();
        let ty = self.proven.get(ident.as_str())?;
        // Only BY-VALUE scalar idents: `&ident` on a `&str`/`&String` param
        // is a double reference — refuse (narrower than clippy).
        let by_value_copy = [
            "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128",
            "isize", "f32", "f64", "bool", "char",
        ];
        if !by_value_copy.contains(&ty.as_str()) {
            return None;
        }
        // Admissible path spellings: [Clone, clone], [std|core, clone,
        // Clone, clone], and [Ty, clone] (Ty == the proven type). The
        // `<Ty as Clone>::clone` spelling parses WITH qself, which the
        // Expr::Path shape already excluded — it refuses by construction
        // (the documented narrower-than-clippy divergence).
        let ok = match segs.as_slice() {
            [c_name, m_name] if m_name == "clone" && (c_name == "Clone" || c_name == ty) => true,
            [root, mod_name, c_name, m_name]
                if (root == "std" || root == "core")
                    && mod_name == "clone"
                    && c_name == "Clone"
                    && m_name == "clone" =>
            {
                true
            }
            _ => false,
        };
        if !ok {
            return None;
        }
        if drops_interior_comment(self.source, byte_range(c), &[byte_range(arg)]) {
            return None;
        }
        Some(ident)
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); x }",
        )
        .is_none());
    }

    #[test]
    fn clone_on_copy_primitive_drops() {
        let out = propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); x.clone() }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(0); x }"), "clone not dropped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn clone_on_str_ref_drops() {
        // `&str` is Copy — the shared-reference spelling the scalar
        // derivation admits.
        let out = propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, s: &str) -> &str { hits.push(0); s.clone() }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(0); s }"), "clone not dropped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn clone_on_string_refuses() {
        // `String` is not Copy — the derivation refuses by construction
        // (`redundant_clone`'s territory, never ours).
        assert!(propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, s: String) -> String { hits.push(0); s.clone() }",
        )
        .is_none());
    }

    #[test]
    fn unproven_receiver_refuses() {
        // The receiver is a call result, not a proven param — refuse.
        assert!(propose_fix(
            "clone_on_copy",
            "fn mk() -> u32 { 3 } fn f(hits: &mut Vec<u8>) -> u32 { hits.push(0); mk().clone() }",
        )
        .is_none());
    }

    #[test]
    fn field_receiver_refuses() {
        // A field access is not a bare proven ident — refuse (v1 stays on
        // the scalar derivation).
        assert!(propose_fix(
            "clone_on_copy",
            "struct S { n: u32 } fn f(hits: &mut Vec<u8>, s: &S) -> u32 { hits.push(0); s.n.clone() }",
        )
        .is_none());
    }

    #[test]
    fn args_method_refuses() {
        // `clone` with an argument is not the no-arg Clone method.
        assert!(propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); x.clone(1) }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); x /* keep */ .clone() }",
        )
        .is_none());
    }

    #[test]
    fn impl_method_param_proven() {
        let out = propose_fix(
            "clone_on_copy",
            "struct S; impl S { fn f(&self, x: u64, hits: &mut Vec<u8>) -> u64 { hits.push(0); x.clone() } }",
        )
        .expect("matched");
        assert!(out.contains("x } }"), "clone not dropped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_in_condition_fires() {
        let out = propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); if x.clone() > 1 { 1 } else { 0 } }",
        )
        .expect("matched");
        assert!(out.contains("if x > 1"), "condition not simplified: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    // ── Batch 230: the UFCS path-call rider (clippy 1.99 #16972) ──

    #[test]
    fn ufcs_trait_path_drops() {
        let out = propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); Clone::clone(&x) }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(0); x }"), "UFCS clone not dropped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn ufcs_type_path_drops() {
        let out = propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); u32::clone(&x) }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(0); x }"), "UFCS type-path clone not dropped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn ufcs_std_qualified_drops() {
        let out = propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, x: u64) -> u64 { hits.push(0); std::clone::Clone::clone(&x) }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(0); x }"), "std-qualified UFCS clone not dropped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn ufcs_wrong_type_path_refuses() {
        // `u64::clone(&x)` where x is u32 — the path names a DIFFERENT type
        // than the proven one; refuse (conservative).
        assert!(propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); u64::clone(&x) }",
        )
        .is_none());
    }

    #[test]
    fn ufcs_by_ref_arg_refuses() {
        // `Clone::clone(s)` with `s: &str` — a by-ref argument spelling,
        // not `&ident`; refuse (narrower than clippy by design).
        assert!(propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, s: &str) -> &str { hits.push(0); Clone::clone(s) }",
        )
        .is_none());
    }

    #[test]
    fn ufcs_double_ref_arg_refuses() {
        // `Clone::clone(&s)` with `s: &str` — `&s` is `&&str`; the
        // by-value-Copy derivation does not cover it.
        assert!(propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, s: &str) -> &str { hits.push(0); Clone::clone(&s) }",
        )
        .is_none());
    }

    #[test]
    fn ufcs_mut_ref_arg_refuses() {
        assert!(propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); Clone::clone(&mut x) }",
        )
        .is_none());
    }

    #[test]
    fn ufcs_two_args_refuses() {
        assert!(propose_fix(
            "clone_on_copy",
            "fn f(hits: &mut Vec<u8>, x: u32, y: u32) -> u32 { hits.push(0); Clone::clone(&x, &y) }",
        )
        .is_none());
    }
}

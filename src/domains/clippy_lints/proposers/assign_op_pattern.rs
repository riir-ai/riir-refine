//! `clippy::assign_op_pattern` — `y = y + 1` → `y += 1` (+ the
//! commutative swapped twin `y = 1 + y` → `y += 1`, `+`/`*` only)
//! (Batch 160 / Plan 151 — the frontier-lead mining pack VI; family fixture
//! verified to fire exactly this lint under all+pedantic+nursery AND the
//! default-groups arm, 2026-09-18; floats/bitops/shifts/params/mut-param
//! arms + the swapped boundary measured by the plan151 probe).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! For BUILT-IN ops on primitives, compound assignment is DEFINED as the
//! binary op + store — same op, same overflow checks, same wrapping — so
//! value-identity is construction. The overloaded-operator hole (a custom
//! `Add`/`AddAssign` pair may differ) is closed by TYPE DERIVATION: the LHS
//! ident must be proven an ARITHMETIC scalar (`f32`/`f64` + the 10 int
//! primitives), where every op in the set is builtin by construction.
//!
//! The evaluation-order swap (the y-read moves after the RHS operand in
//! the compound form) is unobservable: a Copy scalar read has no effects,
//! and an operand that could observe `y` concurrently would need `&mut y`
//! — a borrow error in BOTH forms.
//!
//! The swapped operand `y = b OP y` fires the lint ONLY for COMMUTATIVE
//! ops (probe-measured: `y = 1 + y`/`y = 2 * y` fire, `y = 1 - y` is
//! SILENT) — the proposer mirrors exactly that boundary: `b + y` ≡ `y + b`
//! ≡ `y += b` on integers and IEEE floats (addition/multiplication are
//! commutative), while `b - y` → `y -= b` would be different math and is
//! refused.
//!
//! ## The derivation (exactly-once bindings)
//!
//! A name is PROVEN when it is bound EXACTLY ONCE in the whole fn
//! (params + lets + match/closure/for bindings, NESTED ITEMS INCLUDED —
//! a second binding anywhere kills it) and its type derives from one of:
//! (a) a fn/impl-method param annotation `mut y: i32`;
//! (b) an annotated local `let mut y: i32 = ..`;
//! (c) an unannotated local `let mut y = <already-proven ident>` (the
//!     init-transfer leg — in source order, so the init resolves to the
//!     one binding the source name has);
//! (d) an unannotated local `let mut y = 0i32` (SUFFIXED literal only —
//!     a bare `0` is the `{integer}` ambiguity class, and a bare `0.0`
//!     can still unify with `f32` through a later constraint, so both
//!     refuse).
//!
//! Assignments never change a local's type, so plain/compound assignment
//! sites do not kill the derivation — only RE-BINDINGS do (the pat-ident
//! count). A span-order guard requires the assignment to sit after the
//! local's binding (closes the module-shadow-before-let hole: a use before
//! the let resolves to an outer binding, not ours), and the matcher SKIPS
//! nested items entirely (fns do not capture — an inner fn's `y` is a
//! different variable; the inner fn gets its own pass).

use std::collections::BTreeMap;

use syn::visit::{self, Visit};
use syn::{BinOp, Expr, ExprAssign, File, ImplItemFn, ItemFn, Local};

use super::helpers::{byte_range, drops_interior_comment};
use super::{edit, SourceEdit};

/// The arithmetic scalars — every op in the assign-op set is builtin on
/// these by construction (`bool` refuses: its `&`/`|`/`^` shapes are real
/// but vanishingly rare and outside the probe-verified surface).
const ARITH_SCALARS: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32",
    "f64",
];

/// The emit spelling per op (same order as the `op_of` match).
fn op_assign_str(op: &BinOp) -> Option<&'static str> {
    Some(match op {
        BinOp::Add(_) => "+=",
        BinOp::Sub(_) => "-=",
        BinOp::Mul(_) => "*=",
        BinOp::Div(_) => "/=",
        BinOp::Rem(_) => "%=",
        BinOp::BitAnd(_) => "&=",
        BinOp::BitOr(_) => "|=",
        BinOp::BitXor(_) => "^=",
        BinOp::Shl(_) => "<<=",
        BinOp::Shr(_) => ">>=",
        _ => return None,
    })
}

/// Is `op` one of the two commutative arithmetic ops (the swapped-operand
/// arm's boundary — probe-measured: `y = 1 + y` fires, `y = 1 - y` silent)?
fn is_commutative(op: &BinOp) -> bool {
    matches!(op, BinOp::Add(_) | BinOp::Mul(_))
}

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        proven: BTreeMap::new(),
        saved: BTreeMap::new(),
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

/// `(type text, binding end byte)` — the binding's span end for the
/// assignment-order guard.
struct Proven {
    ty: String,
    end: usize,
}

struct Collector<'a> {
    source: &'a str,
    /// Signature/local-proven arithmetic bindings of the enclosing fn.
    proven: BTreeMap<String, Proven>,
    /// The set to restore on fn exit (one-deep save).
    saved: BTreeMap<String, Proven>,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = {
            let mut c = Counter(BTreeMap::new());
            c.visit_item_fn(f);
            derive_bindings(&c.0, f.sig.inputs.iter(), &f.block)
        };
        self.saved = saved;
        visit::visit_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = {
            let mut c = Counter(BTreeMap::new());
            c.visit_impl_item_fn(f);
            derive_bindings(&c.0, f.sig.inputs.iter(), &f.block)
        };
        self.saved = saved;
        visit::visit_impl_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    /// No `visit_item` skip: a nested `Item::Fn` gets its OWN pass through
    /// `visit_item_fn` (take/restore scopes it — its assignments match
    /// against ITS derivation only, and fns cannot capture outer locals,
    /// so the outer map never leaks in). Only the outer fn's COUNTER sees
    /// the inner bindings (an inner same-name param kills the outer
    /// derivation — the over-conservative direction).
    fn visit_expr_assign(&mut self, a: &'ast ExprAssign) {
        visit::visit_expr_assign(self, a);
        if let Some(replacement) = self.try_rewrite(a) {
            self.edits.push(edit(a, replacement));
        }
    }
}

/// Phase 1 helper: the pat-ident binding counter (whole-fn walk, nested
/// items included).
struct Counter(BTreeMap<String, usize>);

impl<'ast> Visit<'ast> for Counter {
    fn visit_pat_ident(&mut self, p: &'ast syn::PatIdent) {
        *self.0.entry(p.ident.to_string()).or_insert(0) += 1;
        visit::visit_pat_ident(self, p);
    }
}

/// Phase 2's let-walker: records derivable lets in visit (= source)
/// order into the caller's proven map, skipping nested items AND
/// closures (closure-local lets are out of scope; an outer assignment
/// captured inside a closure is still matched by the outer fn's own
/// walk — `visit_expr_closure` is NOT skipped there).
struct LetScan<'p> {
    proven: &'p mut BTreeMap<String, Proven>,
    counts: &'p BTreeMap<String, usize>,
}

impl<'ast> Visit<'ast> for LetScan<'_> {
    fn visit_item(&mut self, _i: &'ast syn::Item) {}
    fn visit_expr_closure(&mut self, _c: &'ast syn::ExprClosure) {}
    fn visit_local(&mut self, l: &'ast Local) {
        visit::visit_local(self, l);
        // Annotated lets fold the type into `Pat::Type` — unwrap to
        // the inner ident + annotation.
        let (pi, anno) = match &l.pat {
            syn::Pat::Ident(pi) => (pi, None),
            syn::Pat::Type(pt) => match &*pt.pat {
                syn::Pat::Ident(pi) => (pi, Some(&*pt.ty)),
                _ => return,
            },
            _ => return,
        };
        if pi.by_ref.is_some() || pi.subpat.is_some() {
            return;
        }
        // (b) annotated let.
        if let Some(ty) = anno.and_then(bare_arith_type) {
            insert_proven(
                self.proven,
                self.counts,
                pi.ident.to_string(),
                ty,
                byte_range(l).end,
            );
            return;
        }
        let Some(init) = l.init.as_ref().map(|i| &*i.expr) else {
            return;
        };
        // (c) init-transfer from an already-proven ident.
        if let Some(src_ty) = ident_text(init)
            .and_then(|n| self.proven.get(&n))
            .map(|p| p.ty.clone())
        {
            insert_proven(
                self.proven,
                self.counts,
                pi.ident.to_string(),
                src_ty,
                byte_range(l).end,
            );
            return;
        }
        // (d) SUFFIXED int/float literal init (bare refuses).
        if let Some(ty) = suffixed_arith_literal(init) {
            insert_proven(
                self.proven,
                self.counts,
                pi.ident.to_string(),
                ty,
                byte_range(l).end,
            );
        }
    }
}

/// Phase 2: the proven map — params first, then lets in SOURCE order (the
/// init-transfer leg looks back at already-proven names only). Skips
/// nested items AND closures (see [`LetScan`]).
fn derive_bindings<'a>(
    counts: &BTreeMap<String, usize>,
    inputs: impl Iterator<Item = &'a syn::FnArg>,
    block: &syn::Block,
) -> BTreeMap<String, Proven> {
    let mut proven: BTreeMap<String, Proven> = BTreeMap::new();
    // Params: annotation only.
    for arg in inputs {
        let syn::FnArg::Typed(p) = arg else { continue };
        let syn::Pat::Ident(pi) = &*p.pat else {
            continue;
        };
        if pi.by_ref.is_some() || pi.subpat.is_some() {
            continue;
        }
        if let Some(ty) = bare_arith_type(&p.ty) {
            insert_proven(
                &mut proven,
                counts,
                pi.ident.to_string(),
                ty,
                byte_range(p).end,
            );
        }
    }
    // Lets, in source order (the walker visits in syntactic order).
    let mut scan = LetScan {
        proven: &mut proven,
        counts,
    };
    scan.visit_block(block);
    proven
}

/// Insert only when the name is bound exactly once (the soundness leg: a
/// single binding means every compiling use of the name in the fn resolves
/// to it — with the span-order guard, uses after the binding have exactly
/// the derived type).
fn insert_proven(
    proven: &mut BTreeMap<String, Proven>,
    counts: &BTreeMap<String, usize>,
    name: String,
    ty: String,
    end: usize,
) {
    if counts.get(&name).is_some_and(|c| *c == 1) {
        proven.insert(name, Proven { ty, end });
    }
}

/// The bare single-segment ARITH path type text (`i32`, `f64`, ...).
fn bare_arith_type(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(p) = ty else { return None };
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    let seg = p.path.segments.first()?;
    if !seg.arguments.is_none() {
        return None;
    }
    let id = seg.ident.to_string();
    ARITH_SCALARS.contains(&id.as_str()).then_some(id)
}

/// A SUFFIXED int/float literal's type (bare literals refuse: `0` is the
/// `{integer}` ambiguity class; `0.0` can still unify with `f32` through a
/// later constraint).
fn suffixed_arith_literal(e: &Expr) -> Option<String> {
    let Expr::Lit(l) = e else { return None };
    match &l.lit {
        syn::Lit::Int(i) => {
            let sfx = i.suffix();
            ARITH_SCALARS
                .iter()
                .find(|t| **t == sfx)
                .map(ToString::to_string)
        }
        syn::Lit::Float(f) => {
            let sfx = f.suffix();
            (sfx == "f32" || sfx == "f64").then(|| sfx.to_string())
        }
        _ => None,
    }
}

/// The bare ident of a single-segment plain path (the per-file ident
/// helper pattern).
fn ident_text(e: &Expr) -> Option<String> {
    let Expr::Path(p) = e else { return None };
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    if !p.path.segments[0].arguments.is_none() {
        return None;
    }
    Some(p.path.segments[0].ident.to_string())
}

impl Collector<'_> {
    fn try_rewrite(&self, a: &ExprAssign) -> Option<String> {
        // LHS: the bare proven ident.
        let y = ident_text(&a.left)?;
        let proven = self.proven.get(&y)?;
        if !ARITH_SCALARS.contains(&proven.ty.as_str()) {
            return None;
        }
        if byte_range(&a.left).start < proven.end {
            return None;
        }
        if !a.attrs.is_empty() {
            return None;
        }
        // RHS: a binary of the admitted op set (outer parens unwrapped).
        let Expr::Binary(b) = unwrap_parens(&a.right) else {
            return None;
        };
        if !b.attrs.is_empty() {
            return None;
        }
        let op = op_assign_str(&b.op)?;
        // Case A: `y = y OP b` → `y OP= b` (operand = right).
        // Case B: `y = b OP y` for COMMUTATIVE ops only (probe-measured
        // boundary) → `y OP= b` (operand = left).
        let operand = if ident_text(&b.left).as_deref() == Some(y.as_str()) {
            &b.right
        } else if is_commutative(&b.op) && ident_text(&b.right).as_deref() == Some(y.as_str()) {
            &b.left
        } else {
            return None;
        };
        // Comments inside the deleted glue refuse; the operand's span is
        // the keep-list (the LHS ident + op are re-spelled).
        if drops_interior_comment(self.source, byte_range(a), &[byte_range(operand)]) {
            return None;
        }
        let operand_text = self.source.get(byte_range(operand))?;
        Some(format!("{y} {op} {operand_text}"))
    }
}

fn unwrap_parens(mut e: &Expr) -> &Expr {
    while let Expr::Paren(p) = e {
        e = &p.expr;
    }
    e
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn local_init_transfer_fires() {
        // The family fixture: unannotated local, init from a proven param.
        let out = propose_fix(
            "assign_op_pattern",
            "fn bump(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y = y + 1; y }",
        )
        .expect("matched");
        assert!(out.contains("y += 1"), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn all_op_classes_fire() {
        for (op, compound) in [
            ("+", "+="),
            ("-", "-="),
            ("*", "*="),
            ("/", "/="),
            ("%", "%="),
            ("&", "&="),
            ("|", "|="),
            ("^", "^="),
            ("<<", "<<="),
            (">>", ">>="),
        ] {
            let out = propose_fix(
                "assign_op_pattern",
                &format!(
                    "fn f(x: u32, hits: &mut Vec<u32>) -> u32 {{ hits.push(x); let mut y = x; y = y {op} 3; y }}",
                ),
            )
            .unwrap_or_else(|| panic!("{op} did not fire"));
            assert!(
                out.contains(&format!("y {compound} 3")),
                "{op} not rewritten to {compound}: {out}"
            );
            syn::parse_file(&out).expect("re-parses");
        }
    }

    #[test]
    fn float_receiver_fires() {
        let out = propose_fix(
            "assign_op_pattern",
            "fn z(x: f64, hits: &mut Vec<f64>) -> f64 { hits.push(x); let mut y = x; y = y + 1.0; y }",
        )
        .expect("matched");
        assert!(out.contains("y += 1.0"), "float not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn swapped_commutative_fires() {
        let out = propose_fix(
            "assign_op_pattern",
            "fn s(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y = 1 + y; y }",
        )
        .expect("matched");
        assert!(out.contains("y += 1"), "swapped add not rewritten: {out}");
        let out = propose_fix(
            "assign_op_pattern",
            "fn s(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y = 2 * y; y }",
        )
        .expect("matched");
        assert!(out.contains("y *= 2"), "swapped mul not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn swapped_non_commutative_refuses() {
        // `y = 1 - y` → `y -= 1` would be different math — clippy is
        // silent here (probe-measured) and so is the proposer.
        assert!(propose_fix(
            "assign_op_pattern",
            "fn s(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y = 1 - y; y }",
        )
        .is_none());
    }

    #[test]
    fn annotated_and_suffixed_lets_fire() {
        let out = propose_fix(
            "assign_op_pattern",
            "fn f(hits: &mut Vec<i64>) -> i64 { hits.push(0); let mut y: i64 = 0; y = y + 5; y }",
        )
        .expect("matched");
        assert!(out.contains("y += 5"), "annotated let not rewritten: {out}");
        let out = propose_fix(
            "assign_op_pattern",
            "fn f(hits: &mut Vec<u8>) -> u8 { hits.push(0); let mut y = 3u8; y = y + 1; y }",
        )
        .expect("matched");
        assert!(out.contains("y += 1"), "suffixed init not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn mut_param_shape_fires() {
        let out = propose_fix(
            "assign_op_pattern",
            "fn grow(mut p: i32, hits: &mut Vec<i32>) -> i32 { hits.push(p); p = p + 5; p }",
        )
        .expect("matched");
        assert!(out.contains("p += 5"), "param shape not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn paren_rhs_fires() {
        let out = propose_fix(
            "assign_op_pattern",
            "fn f(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y = (y + 1); y }",
        )
        .expect("matched");
        assert!(out.contains("y += 1"), "paren rhs not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn chained_init_transfer_fires() {
        // Two hops: param → a → y.
        let out = propose_fix(
            "assign_op_pattern",
            "fn f(x: u16, hits: &mut Vec<u16>) -> u16 { hits.push(x); let a = x; let mut y = a; y = y * 2; y }",
        )
        .expect("matched");
        assert!(out.contains("y *= 2"), "chained derivation failed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn unsuffixed_inits_refuse() {
        // `{integer}` ambiguity class — bare literals refuse.
        assert!(propose_fix(
            "assign_op_pattern",
            "fn f(hits: &mut Vec<i32>) -> i32 { hits.push(0); let mut y = 0; y = y + 1; y }",
        )
        .is_none());
        // Bare float can still unify with f32 through a later constraint.
        assert!(propose_fix(
            "assign_op_pattern",
            "fn f(hits: &mut Vec<f64>) -> f64 { hits.push(0.0); let mut y = 0.0; y = y + 1.0; y }",
        )
        .is_none());
    }

    #[test]
    fn generic_and_unproven_refuse() {
        assert!(propose_fix(
            "assign_op_pattern",
            "fn f<T: Copy + std::ops::Add<Output = T>>(mut y: T, one: T) -> T { y = y + one; y }",
        )
        .is_none());
        // bool: outside ARITH_SCALARS (the documented divergence).
        assert!(propose_fix(
            "assign_op_pattern",
            "fn f(hits: &mut Vec<bool>) { hits.push(false); let mut b = true; b = b & true; }",
        )
        .is_none());
    }

    #[test]
    fn logops_refuse() {
        // `&&`/`||` have no compound form.
        assert!(propose_fix(
            "assign_op_pattern",
            "fn f(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); let mut b = x; b = b && x; b }",
        )
        .is_none());
    }

    #[test]
    fn rebound_ident_refuses() {
        // A second binding of `y` anywhere kills the derivation.
        assert!(propose_fix(
            "assign_op_pattern",
            "fn f(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; let y = 9i32; y = y + 1; y }",
        )
        .is_none());
        // ...including a closure param rebind.
        assert!(propose_fix(
            "assign_op_pattern",
            "fn f(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; let g = |y: i32| y + 1; y = y + 1; g(1) }",
        )
        .is_none());
    }

    #[test]
    fn nested_fn_gets_its_own_pass() {
        // The inner fn's local is derived + rewritten by the inner pass;
        // the outer fn's map is untouched.
        let out = propose_fix(
            "assign_op_pattern",
            "fn outer(hits: &mut Vec<i32>) -> i32 { hits.push(0); fn inner(q: u32) -> u32 { let mut z = q; z = z + 7; z } inner(1) }",
        )
        .expect("matched");
        assert!(
            out.contains("z += 7"),
            "inner fn pattern not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn comment_in_glue_refuses() {
        assert!(propose_fix(
            "assign_op_pattern",
            "fn f(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y = y + /* one */ 1; y }",
        )
        .is_none());
    }

    #[test]
    fn impl_method_param_derivation() {
        let out = propose_fix(
            "assign_op_pattern",
            "struct W { n: u8 }\nimpl W {\n    fn grow(&self, mut c: u8) -> u8 { c = c * 2; c }\n}",
        )
        .expect("matched");
        assert!(
            out.contains("c *= 2"),
            "impl-method param not derived: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }
}

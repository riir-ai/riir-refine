//! `clippy::ptr_offset_with_cast` — `p.offset(i as isize)` → `p.add(i)`
//! (+ `wrapping_offset` → `wrapping_add`) (Batch 184 / Plan 179 — the
//! coverage-report pack XVI; the Plan-169 staged IOU landed. Probe:
//! `scripts/plan179_pack16_probe.py`).
//!
//! ## Why the rewrite is construction-identity (the FIXABLE bar)
//!
//! For a raw pointer `p: *T` and `i: usize`, `p.add(i)` is exactly
//! `p.offset(i as isize)` — std's own definition (`offset` takes
//! `isize` and adds; `add` takes `usize` and cannot go backwards). The
//! probe measured upstream's own shape bound: the lint FIRES only on the
//! usize→isize cast (`powc_u64` with `i: u64` is SILENT; the negative
//! `-(i as isize)` arm is SILENT — the lint matches the `ExprCast` node,
//! which unary-minus wraps), so our operand guard is upstream PARITY,
//! not a divergence. `wrapping_offset(i as isize)` maps to
//! `wrapping_add(i)` by the same identity (measured suggestion text).
//!
//! ## Guards (the bounded fix-space — mirrors upstream's typeck)
//!
//! - **Receiver provenance is POINTER-VISIBLE only** — the receiver must
//!   be a bare ident proven a raw pointer either by the enclosing fn
//!   head (`p: *const T` / `*mut T` param — the
//!   `scalar_param_heads_over` never-rebound law) or by a local whose
//!   sole binding is `<e>.as_ptr()` / `.as_mut_ptr()` (zero-arg,
//!   no turbofish). Upstream typecks the receiver (probe:
//!   `powc_custom_offset` — a custom `offset` method is SILENT), so the
//!   guard is parity where syntax can see it and the documented
//!   narrower-than-clippy divergence where it cannot (a pointer handed
//!   through an unannotated helper call refuses). Locals with any other
//!   binding shape, any re-binding (`bind_count != 1`), or any
//!   assignment to the ident refuse.
//! - **Operand provenance is usize-param only** — the cast's operand
//!   must be a bare ident param with declared type exactly `usize`
//!   (`usize` is upstream's own firing precondition, measured).
//!   Literals, call results, `u64` operands, and annotated lets refuse.
//! - The method must be exactly `offset` or `wrapping_offset`, one arg,
//!   no turbofish, no attrs; `offset_from` is not the lint's cast shape.
//! - Comments inside the replaced span refuse (the emitter would delete
//!   them outright).
//! - The edit is a single whole-method-call splice — both receiver and
//!   operand are proven bare idents, so the caller's formatting is
//!   preserved byte-identically everywhere else.

use std::collections::{BTreeMap, BTreeSet};

use syn::visit::Visit;
use syn::{Block, Expr, ExprCast, ExprMethodCall, File, ImplItemFn, ItemFn, Pat};

use super::helpers::{body_rebinds, byte_range, drops_interior_comment};
use super::{edit, SourceEdit};

/// The syntax-visible provenance of a bare ident: a raw pointer, or a
/// `usize` scalar (upstream's own firing precondition, measured).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prov {
    Ptr,
    Usize,
}

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        prov: BTreeMap::new(),
        saved: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Syntax-proven bindings of the enclosing fn (`ident → Prov`).
    prov: BTreeMap<String, Prov>,
    /// The stack to restore on fn exit (one-deep in practice).
    saved: Vec<BTreeMap<String, Prov>>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        let saved = std::mem::take(&mut self.prov);
        self.saved.push(saved);
        self.prov = fn_provenance(f.sig.inputs.iter(), &f.block);
        syn::visit::visit_item_fn(self, f);
        self.prov = self.saved.pop().unwrap_or_default();
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        let saved = std::mem::take(&mut self.prov);
        self.saved.push(saved);
        self.prov = fn_provenance(f.sig.inputs.iter(), &f.block);
        syn::visit::visit_impl_item_fn(self, f);
        self.prov = self.saved.pop().unwrap_or_default();
    }

    fn visit_expr_method_call(&mut self, mc: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, mc);
        if let Some(e) = self.try_rewrite(mc) {
            self.edits.push(e);
        }
    }
}

impl Collector<'_> {
    fn try_rewrite(&self, mc: &ExprMethodCall) -> Option<SourceEdit> {
        let new_method = match mc.method.to_string().as_str() {
            "offset" => "add",
            "wrapping_offset" => "wrapping_add",
            _ => return None,
        };
        if mc.args.len() != 1 || mc.turbofish.is_some() || !mc.attrs.is_empty() {
            return None;
        }
        let recv = path_ident(&mc.receiver)?;
        if self.prov.get(recv.as_str()) != Some(&Prov::Ptr) {
            return None;
        }
        let Expr::Cast(ExprCast { expr, ty, .. }) = mc.args.first()? else {
            return None;
        };
        if !cast_target_is_isize(ty) {
            return None;
        }
        let operand = path_ident(expr)?;
        if self.prov.get(operand.as_str()) != Some(&Prov::Usize) {
            return None;
        }
        if drops_interior_comment(self.source, byte_range(mc), &[]) {
            return None;
        }
        Some(edit(mc, format!("{recv}.{new_method}({operand})")))
    }
}

/// The syntax-proven binding map of one fn: pointer-typed params, `usize`
/// params, and locals whose sole binding is an `as_ptr`/`as_mut_ptr`
/// initializer. Every arm carries the never-rebound law: a param with any
/// body rebind (shadow `let`, `=`, or compound-assign) refuses via
/// [`body_rebinds`]; a local refuses unless its ident appears as exactly
/// ONE pattern in the whole fn (its own `let`) and is never assigned.
fn fn_provenance<'i>(
    inputs: impl Iterator<Item = &'i syn::FnArg>,
    block: &Block,
) -> BTreeMap<String, Prov> {
    let mut out = BTreeMap::new();
    for arg in inputs {
        let syn::FnArg::Typed(p) = arg else { continue };
        let Pat::Ident(pi) = &*p.pat else { continue };
        if pi.by_ref.is_some() || pi.subpat.is_some() {
            continue;
        }
        let prov = match &*p.ty {
            syn::Type::Ptr(_) => Prov::Ptr,
            syn::Type::Path(tp)
                if tp.qself.is_none()
                    && tp.path.leading_colon.is_none()
                    && tp.path.segments.len() == 1
                    && tp.path.segments[0].arguments.is_none()
                    && tp.path.segments[0].ident == "usize" =>
            {
                Prov::Usize
            }
            _ => continue,
        };
        if body_rebinds(block, &pi.ident.to_string()) {
            continue;
        }
        out.insert(pi.ident.to_string(), prov);
    }
    let scan = Scan::new(block);
    for ident in scan.as_ptr_locals {
        // The let's own pattern is the ident's ONLY PatIdent in the fn,
        // and nothing ever assigns to it — the initializer is the
        // construction fact.
        if scan.binds.get(&ident).copied() == Some(1) && !scan.assigned.contains(&ident) {
            out.insert(ident, Prov::Ptr);
        }
    }
    out
}

/// One conservative pre-pass over a fn body: pattern-binding counts,
/// assignment targets, and the `as_ptr`-initialized locals.
struct Scan {
    binds: BTreeMap<String, u32>,
    assigned: BTreeSet<String>,
    as_ptr_locals: BTreeSet<String>,
}

impl Scan {
    fn new(block: &Block) -> Scan {
        let mut s = Scan {
            binds: BTreeMap::new(),
            assigned: BTreeSet::new(),
            as_ptr_locals: BTreeSet::new(),
        };
        s.visit_block(block);
        s
    }
}

impl<'ast> Visit<'ast> for Scan {
    fn visit_pat_ident(&mut self, p: &'ast syn::PatIdent) {
        *self.binds.entry(p.ident.to_string()).or_default() += 1;
        syn::visit::visit_pat_ident(self, p);
    }

    fn visit_expr_assign(&mut self, a: &'ast syn::ExprAssign) {
        if let Some(id) = path_ident(&a.left) {
            self.assigned.insert(id);
        }
        syn::visit::visit_expr_assign(self, a);
    }

    fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
        use syn::BinOp;
        if matches!(
            b.op,
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
        ) {
            if let Some(id) = path_ident(&b.left) {
                self.assigned.insert(id);
            }
        }
        syn::visit::visit_expr_binary(self, b);
    }

    fn visit_local(&mut self, l: &'ast syn::Local) {
        if let Pat::Ident(pi) = &l.pat {
            if pi.by_ref.is_none() && pi.subpat.is_none() {
                if let Some(init) = &l.init {
                    if is_ptr_ctor_call(&init.expr) {
                        self.as_ptr_locals.insert(pi.ident.to_string());
                    }
                }
            }
        }
        syn::visit::visit_local(self, l);
    }
}

/// `<e>.as_ptr()` / `<e>.as_mut_ptr()` — zero-arg, no turbofish, no
/// attrs (the std raw-pointer constructors; the only initializer
/// spellings whose pointer-ness is syntax-visible).
fn is_ptr_ctor_call(e: &Expr) -> bool {
    let Expr::MethodCall(mc) = e else { return false };
    matches!(mc.method.to_string().as_str(), "as_ptr" | "as_mut_ptr")
        && mc.args.is_empty()
        && mc.turbofish.is_none()
        && mc.attrs.is_empty()
}

/// The single-segment plain-ident spelling of a path expression.
fn path_ident(e: &Expr) -> Option<String> {
    let Expr::Path(p) = e else { return None };
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    if !p.path.segments[0].arguments.is_none() {
        return None;
    }
    Some(p.path.segments[0].ident.to_string())
}

/// Is the cast target the plain `isize` path (pointer targets,
/// multi-segment paths, generics refuse)?
fn cast_target_is_isize(ty: &syn::Type) -> bool {
    let syn::Type::Path(tp) = ty else { return false };
    if tp.qself.is_some() || tp.path.leading_colon.is_some() || tp.path.segments.len() != 1 {
        return false;
    }
    let seg = &tp.path.segments[0];
    seg.arguments.is_none() && seg.ident == "isize"
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            "ptr_offset_with_cast",
            "fn f(hits: &mut Vec<u8>, b: &[u8], i: usize) -> u8 { hits.push(0); let p = b.as_ptr(); unsafe { *p.add(i) } }",
        )
        .is_none());
    }

    #[test]
    fn plain_as_ptr_local_rewrites() {
        let out = propose_fix(
            "ptr_offset_with_cast",
            "pub fn f(b: &[u8], i: usize) -> u8 {\n    let p = b.as_ptr();\n    unsafe { *p.offset(i as isize) }\n}",
        )
        .expect("matched");
        assert!(out.contains("unsafe { *p.add(i) }"), "{out}");
        assert!(!out.contains("as isize"), "cast must drop: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn wrapping_maps_to_wrapping_add() {
        let out = propose_fix(
            "ptr_offset_with_cast",
            "pub fn f(b: &[u8], i: usize) -> u8 {\n    let p = b.as_ptr();\n    unsafe { *p.wrapping_offset(i as isize) }\n}",
        )
        .expect("matched");
        assert!(out.contains("*p.wrapping_add(i)"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn param_ptr_provenance_rewrites() {
        let out = propose_fix(
            "ptr_offset_with_cast",
            "pub fn g(p: *const u8, i: usize) -> u8 { unsafe { *p.offset(i as isize) } }",
        )
        .expect("matched");
        assert!(out.contains("*p.add(i)"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn impl_method_param_proven() {
        let out = propose_fix(
            "ptr_offset_with_cast",
            "struct S; impl S { fn f(&self, p: *mut u8, i: usize) -> u8 { unsafe { *p.offset(i as isize) } } }",
        )
        .expect("matched");
        assert!(out.contains("*p.add(i)"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn as_mut_ptr_local_proven() {
        let out = propose_fix(
            "ptr_offset_with_cast",
            "pub fn f(b: &mut [u8], i: usize) -> u8 { let p = b.as_mut_ptr(); unsafe { *p.offset(i as isize) } }",
        )
        .expect("matched");
        assert!(out.contains("*p.add(i)"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn u64_operand_refuses() {
        // Upstream is SILENT here (measured: the firing precondition is
        // usize) — parity guard.
        assert!(propose_fix(
            "ptr_offset_with_cast",
            "pub fn f(b: &[u8], i: u64) -> u8 { let p = b.as_ptr(); unsafe { *p.offset(i as isize) } }",
        )
        .is_none());
    }

    #[test]
    fn isize_operand_refuses() {
        // Already-isize offsets carry no cast — and the identity cast
        // spelling is unnecessary_cast's territory.
        assert!(propose_fix(
            "ptr_offset_with_cast",
            "pub fn f(b: &[u8], i: isize) -> u8 { let p = b.as_ptr(); unsafe { *p.offset(i as isize) } }",
        )
        .is_none());
    }

    #[test]
    fn unproven_local_refuses() {
        // A helper call result has no syntax-visible pointer provenance
        // (the documented narrower-than-clippy divergence).
        assert!(propose_fix(
            "ptr_offset_with_cast",
            "fn mk() -> *const u8 { std::ptr::null() } pub fn f(i: usize) -> u8 { let p = mk(); unsafe { *p.offset(i as isize) } }",
        )
        .is_none());
    }

    #[test]
    fn rebound_local_refuses() {
        // A second pattern binding of the same ident (a shadow `let`)
        // revokes the local's construction fact.
        assert!(propose_fix(
            "ptr_offset_with_cast",
            "pub fn f(b: &[u8], i: usize, other: &[u8]) -> u8 { let p = b.as_ptr(); let p = other.as_ptr(); unsafe { *p.offset(i as isize) } }",
        )
        .is_none());
    }

    #[test]
    fn assigned_local_refuses() {
        // `p = other;` may re-point the local — provenance revoked.
        assert!(propose_fix(
            "ptr_offset_with_cast",
            "pub fn f(b: &[u8], c: &[u8], i: usize) -> u8 { let p = b.as_ptr(); p = c.as_ptr(); unsafe { *p.offset(i as isize) } }",
        )
        .is_none());
    }

    #[test]
    fn rebound_param_refuses() {
        // The param never-rebound law (body_rebinds).
        assert!(propose_fix(
            "ptr_offset_with_cast",
            "pub fn f(b: &[u8], i: usize) -> u8 { let p = b.as_ptr(); let i = i + 1; unsafe { *p.offset(i as isize) } }",
        )
        .is_none());
    }

    #[test]
    fn parenthesized_receiver_refuses() {
        assert!(propose_fix(
            "ptr_offset_with_cast",
            "pub fn f(b: &[u8], i: usize) -> u8 { let p = b.as_ptr(); unsafe { *(p).offset(i as isize) } }",
        )
        .is_none());
    }

    #[test]
    fn field_receiver_refuses() {
        assert!(propose_fix(
            "ptr_offset_with_cast",
            "struct S { p: *const u8 } pub fn f(s: &S, i: usize) -> u8 { unsafe { *s.p.offset(i as isize) } }",
        )
        .is_none());
    }

    #[test]
    fn literal_operand_refuses() {
        assert!(propose_fix(
            "ptr_offset_with_cast",
            "pub fn f(b: &[u8]) -> u8 { let p = b.as_ptr(); unsafe { *p.offset(5 as isize) } }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            "ptr_offset_with_cast",
            "pub fn f(b: &[u8], i: usize) -> u8 { let p = b.as_ptr(); unsafe { *p.offset /* why */ (i as isize) } }",
        )
        .is_none());
    }

    #[test]
    fn non_cast_arg_refuses() {
        assert!(propose_fix(
            "ptr_offset_with_cast",
            "pub fn f(b: &[u8], i: isize) -> u8 { let p = b.as_ptr(); unsafe { *p.offset(i) } }",
        )
        .is_none());
    }

    #[test]
    fn fix_is_fixpoint_stable() {
        let src = "pub fn f(b: &[u8], i: usize) -> u8 {\n    let p = b.as_ptr();\n    unsafe { *p.offset(i as isize) }\n}";
        let first = propose_fix("ptr_offset_with_cast", src).expect("matched");
        assert!(propose_fix("ptr_offset_with_cast", &first).is_none());
    }
}

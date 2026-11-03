//! `clippy::manual_bit_width` (pedantic) + `clippy::mismatched_bit_width_type`
//! (suspicious) — `<Ty>::BITS - x.leading_zeros()` rewritten to the std
//! `bit_width()` spelling chosen by the RECEIVER's proven type (Batch 230 /
//! Plan 232; both lints NEW in clippy 1.99, PR #16902; probe-verified
//! 2026-10-13 against clippy 1.99.0, isolated toolchain).
//!
//! Both lints live in ONE upstream clippy source file
//! (`clippy_lints/src/bit_width.rs`) and fire on the SAME shape — the split
//! is only which bug you have: `manual_bit_width` when the LHS type EQUALS
//! the receiver's type (a manual reimplementation of the new std method),
//! `mismatched_bit_width_type` when it does not (a probable width bug —
//! whose fix STILL keys on the receiver). Because the fix is identical
//! either way, ONE proposer module houses both dispatch names (the
//! `doc_lazy_continuation` one-module-two-lints precedent) and
//! `collect_edits` is lint-name-agnostic: it fires on the shape regardless
//! of which of the two names routed here.
//!
//! ## Why the rewrite is construction-identity (the FIXABLE bar)
//!
//! `bit_width()` is DEFINED as the significant-bit width of the receiver's
//! value — for an unsigned `x: T`, `T::BITS - x.leading_zeros()` is the std
//! definition of x's bit width, so `x.bit_width()` is the same value by
//! construction (and the same `u32` type: `BITS`, `leading_zeros` and
//! `bit_width()` all return `u32`). The fix table (every row
//! compile-verified at clippy 1.99.0 AND rustc 1.98.1):
//!
//! | receiver (proven) | fix |
//! |---|---|
//! | `u8`/`u16`/`u32`/`u64`/`u128`/`usize` | `x.bit_width()` |
//! | `i8`/…/`i128`/`isize` | `x.cast_unsigned().bit_width()` |
//! | `NonZero<uN>`/`NonZeroUxx`/`NonZeroUsize` | `x.bit_width().get()` |
//! | `NonZero<iN>`/`NonZeroIxx`/`NonZeroIsize` | `x.cast_unsigned().bit_width().get()` |
//!
//! `cast_unsigned` reinterprets the two's-complement bit pattern, so the
//! leading-zero count is unchanged; `.get()` unwraps the `NonZero<u32>` the
//! NonZero-receiver form returns. The `mismatched` shape (e.g. a `u32::BITS`
//! LHS over a `u8` receiver) fixes by the RECEIVER the same way — clippy's
//! own suggestion is receiver-driven for both lints (probe-verified).
//!
//! **MSRV note:** `bit_width()` stabilized at 1.97 and the LINT ships in
//! clippy 1.99 — clippy is MSRV-aware, so the lint only fires where the fix
//! compiles (the `chunks_exact_to_as_chunks` precedent). This proposer rides
//! the same assumption: the oracle lane only feeds us spans the lint already
//! fired on, and the compile gate reverts any fix that does not compile.
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - The LHS must be an `Expr::Path` (no qself, no leading colon, at least
//!   one prefix segment) whose LAST segment is exactly `BITS` with no
//!   generic arguments — any prefix spelling passes (`u32`, `NonZeroU32`,
//!   `NonZero::<u32>`, `num::NonZero::<usize>`, `std::num::NonZero::<i32>`);
//!   generics on the BITS segment itself refuse. A literal LHS
//!   (`1_000_000 - …`) and a non-BITS last segment (`u32::MAX - …`) never
//!   match — probe-verified silent upstream too.
//! - The RHS must be exactly `leading_zeros` with ZERO arguments and no
//!   turbofish, on a BARE single-segment path ident receiver. Field, call,
//!   index and method-chain receivers refuse (not a bare ident).
//! - **Provenance is PARAM-only**: the receiver's type must be provable from
//!   the enclosing fn's signature — the integer rows of
//!   `scalar_param_heads_over` plus the local `NonZero` derivation below, both
//!   under the never-rebound guard (zero rebinds make the annotation a
//!   CONSTRUCTION fact). Locals, lets, fields and calls are unprovable —
//!   clippy can type them, we refuse. The `mismatched` shape with an
//!   UNPROVEN receiver refuses for the same reason: the fix KIND is chosen
//!   by the receiver's type, and an unproven receiver leaves no kind to
//!   pick.
//! - Receivers proven to a NON-integer type (`f32`/`f64`/`bool`/`char`, and
//!   the `String`/`&str` rows the scalar derivation admits) refuse — no
//!   `bit_width()` exists there.
//! - The `NonZero` derivation accepts exactly `NonZero<uN>` / `NonZero<iN>` /
//!   `NonZeroUxx` / `NonZeroUsize` / `NonZeroIxx` / `NonZeroIsize`, bare or
//!   `std::num::`-qualified; `core::num::`, deeper qualifiers, qself, a
//!   leading colon, generics on the alias form, and non-int type arguments
//!   refuse.
//! - Comments inside the replaced span refuse (the receiver ident is the
//!   kept fragment; the LHS path and the `.leading_zeros()` wrapper are
//!   dropped).

use std::collections::BTreeMap;

use syn::visit::Visit;
use syn::{BinOp, Expr, ExprBinary, File, ImplItemFn, ItemFn};

use super::helpers::{body_rebinds, byte_range, drops_interior_comment, scalar_param_heads_over};
use super::{edit, SourceEdit};

/// The bare unsigned-int type names (the scalar rows + the `NonZero` generic
/// argument).
const UNSIGNED_INTS: &[&str] = &["u8", "u16", "u32", "u64", "u128", "usize"];

/// The bare signed-int type names (the scalar rows + the `NonZero` generic
/// argument).
const SIGNED_INTS: &[&str] = &["i8", "i16", "i32", "i64", "i128", "isize"];

/// The fixed-width `NonZero` alias idents, unsigned side.
const NONZERO_ALIAS_UNSIGNED: &[&str] = &[
    "NonZeroU8",
    "NonZeroU16",
    "NonZeroU32",
    "NonZeroU64",
    "NonZeroU128",
    "NonZeroUsize",
];

/// The fixed-width `NonZero` alias idents, signed side.
const NONZERO_ALIAS_SIGNED: &[&str] = &[
    "NonZeroI8",
    "NonZeroI16",
    "NonZeroI32",
    "NonZeroI64",
    "NonZeroI128",
    "NonZeroIsize",
];

/// The proven receiver kinds the fix table keys on — the fix is chosen by
/// the RECEIVER's type (clippy's own suggestion keys the same way for both
/// lints, probe-verified), so this is the whole derivation surface.
#[derive(Clone, Copy)]
enum Width {
    /// `u8`/`u16`/`u32`/`u64`/`u128`/`usize` — `x.bit_width()`.
    Unsigned,
    /// `i8`/…/`i128`/`isize` — `x.cast_unsigned().bit_width()`.
    Signed,
    /// `NonZero<uN>`/`NonZeroUxx`/`NonZeroUsize` — `x.bit_width().get()`.
    NonZeroUnsigned,
    /// `NonZero<iN>`/`NonZeroIxx`/`NonZeroIsize` —
    /// `x.cast_unsigned().bit_width().get()`.
    NonZeroSigned,
}

impl Width {
    /// The fix text for a receiver named `id` proven to this kind (the
    /// probe-verified table; every row compile-verified at clippy 1.99.0
    /// AND rustc 1.98.1).
    fn fix(self, id: &str) -> String {
        match self {
            Width::Unsigned => format!("{id}.bit_width()"),
            Width::Signed => format!("{id}.cast_unsigned().bit_width()"),
            Width::NonZeroUnsigned => format!("{id}.bit_width().get()"),
            Width::NonZeroSigned => format!("{id}.cast_unsigned().bit_width().get()"),
        }
    }
}

/// Collect the lint's edits against the original source. Empty = no match.
///
/// Lint-name-agnostic: both `manual_bit_width` and
/// `mismatched_bit_width_type` route here — the shape and the fix are the
/// same for both; only the lint's NAME differs by whether the LHS type
/// equals the receiver's type, which a parse-only proposer cannot see.
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
    /// Signature-proven receiver kinds of the enclosing fn.
    proven: BTreeMap<String, Width>,
    /// The set to restore on fn exit (one-deep save).
    saved: BTreeMap<String, Width>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = width_params_over(f.sig.inputs.iter(), &f.block);
        self.saved = saved;
        syn::visit::visit_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = width_params_over(f.sig.inputs.iter(), &f.block);
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
        if !matches!(b.op, BinOp::Sub(_)) {
            return None;
        }
        if !lhs_is_bits_path(&b.left) {
            return None;
        }
        let Expr::MethodCall(m) = &*b.right else {
            return None;
        };
        if m.method != "leading_zeros" || !m.args.is_empty() || m.turbofish.is_some() {
            return None;
        }
        let id = bare_ident_of(&m.receiver)?;
        let kind = *self.proven.get(id.as_str())?;
        if drops_interior_comment(self.source, byte_range(b), &[byte_range(&m.receiver)]) {
            return None;
        }
        Some(kind.fix(&id))
    }
}

/// Is the subtraction's LHS a `<Ty>::BITS` path? Any type-prefix spelling
/// passes (`u32`, `NonZeroU32`, `NonZero::<u32>`, `num::NonZero::<usize>`,
/// `std::num::NonZero::<i32>`); qself, a leading colon, a bare lone `BITS`,
/// and generics on the BITS segment itself refuse. A literal or non-path
/// LHS is not a path at all — the `1_000_000 - …` silence upstream mirrors.
fn lhs_is_bits_path(left: &Expr) -> bool {
    let Expr::Path(p) = left else { return false };
    if p.qself.is_some() || p.path.leading_colon.is_some() {
        return false;
    }
    if p.path.segments.len() < 2 {
        return false; // `<Ty>::BITS` always carries a type prefix
    }
    let Some(last) = p.path.segments.last() else {
        return false;
    };
    last.ident == "BITS" && last.arguments.is_none()
}

/// The receiver must be a BARE single-segment path ident (no qself, no
/// leading colon, no generic args) — fields, calls, indexes and method
/// chains refuse (v1 provenance is param-only; see the module guards).
fn bare_ident_of(e: &Expr) -> Option<String> {
    let Expr::Path(p) = e else { return None };
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    if !p.path.segments[0].arguments.is_none() {
        return None;
    }
    Some(p.path.segments[0].ident.to_string())
}

/// Signature-proven bit-width receiver kinds of `f` (param ident → kind):
/// the INTEGER subset of the shared scalar derivation plus the local
/// `NonZero` derivation, both under the never-rebound guard.
fn width_params_over<'i>(
    inputs: impl Iterator<Item = &'i syn::FnArg>,
    block: &syn::Block,
) -> BTreeMap<String, Width> {
    let args: Vec<&syn::FnArg> = inputs.collect();
    let mut out = BTreeMap::new();
    // Scalar rows: keep the integer subset only — a receiver proven to
    // f32/f64/bool/char (or the String/&str rows) has no `bit_width()`.
    for (ident, text) in scalar_param_heads_over(args.iter().copied(), block) {
        if let Some(w) = scalar_width_of(&text) {
            out.insert(ident, w);
        }
    }
    // NonZero rows: the local derivation, same param shape rules as the
    // shared one (plain Pat::Ident, never-rebound).
    for arg in args {
        let syn::FnArg::Typed(p) = arg else { continue };
        let syn::Pat::Ident(pi) = &*p.pat else {
            continue;
        };
        if pi.by_ref.is_some() || pi.subpat.is_some() {
            continue;
        }
        let Some(w) = nonzero_width_of(&p.ty) else {
            continue;
        };
        if body_rebinds(block, &pi.ident.to_string()) {
            continue;
        }
        out.insert(pi.ident.to_string(), w);
    }
    out
}

/// The width kind of a scalar-derivation type text (`"u32"`, `"f64"`, …) —
/// integers map, every other admitted scalar (floats, bool, char, the
/// String rows) has no `bit_width()` and refuses.
fn scalar_width_of(text: &str) -> Option<Width> {
    if UNSIGNED_INTS.contains(&text) {
        Some(Width::Unsigned)
    } else if SIGNED_INTS.contains(&text) {
        Some(Width::Signed)
    } else {
        None
    }
}

/// The `NonZero` row of the local derivation: the param's type must spell
/// EXACTLY one of `NonZero<u8>`/`NonZero<i8>`/…, `NonZeroU8`/`NonZeroI8`/…/
/// `NonZeroUsize`/`NonZeroIsize`, each optionally `std::num::`-qualified
/// (the canonical spellings — the structural equivalent of exact
/// ToTokens-text matching). `core::num::`, deeper qualifiers, qself, a
/// leading colon, generics on the alias form, and non-int type arguments
/// all refuse — the conservative direction.
fn nonzero_width_of(ty: &syn::Type) -> Option<Width> {
    let syn::Type::Path(p) = ty else { return None };
    if p.qself.is_some() || p.path.leading_colon.is_some() {
        return None;
    }
    // Peel one `std::num::` qualifier — the canonical qualified spelling.
    let all: Vec<&syn::PathSegment> = p.path.segments.iter().collect();
    let segs: &[&syn::PathSegment] = match all.as_slice() {
        [a, b, rest @ ..] if is_std_num(a, b) => rest,
        _ => all.as_slice(),
    };
    match segs {
        [s] if s.ident == "NonZero" => generic_nonzero_width(s),
        [s] if s.arguments.is_none() => alias_width_of(&s.ident.to_string()),
        _ => None,
    }
}

/// Are these two path segments exactly `std::num` (both argument-less)?
fn is_std_num(a: &syn::PathSegment, b: &syn::PathSegment) -> bool {
    a.ident == "std" && b.ident == "num" && a.arguments.is_none() && b.arguments.is_none()
}

/// The `NonZero<int>`-spelling arm: exactly one angle-bracketed TYPE
/// argument, itself a bare int path.
fn generic_nonzero_width(s: &syn::PathSegment) -> Option<Width> {
    let syn::PathArguments::AngleBracketed(ab) = &s.arguments else {
        return None;
    };
    if ab.args.len() != 1 {
        return None;
    }
    let Some(syn::GenericArgument::Type(t)) = ab.args.first() else {
        return None;
    };
    let syn::Type::Path(tp) = t else { return None };
    if tp.qself.is_some() || tp.path.leading_colon.is_some() || tp.path.segments.len() != 1 {
        return None;
    }
    let seg = tp.path.segments.first()?;
    if !seg.arguments.is_none() {
        return None;
    }
    nonzero_width_of_int(&seg.ident.to_string())
}

/// The NonZero-receiver width kind of a bare int type name — the generic
/// argument of the `NonZero<T>` spelling.
fn nonzero_width_of_int(name: &str) -> Option<Width> {
    if UNSIGNED_INTS.contains(&name) {
        Some(Width::NonZeroUnsigned)
    } else if SIGNED_INTS.contains(&name) {
        Some(Width::NonZeroSigned)
    } else {
        None
    }
}

/// The `NonZeroU32`-alias arm: exact table membership, unsigned side vs
/// signed side.
fn alias_width_of(ident: &str) -> Option<Width> {
    if NONZERO_ALIAS_UNSIGNED.contains(&ident) {
        Some(Width::NonZeroUnsigned)
    } else if NONZERO_ALIAS_SIGNED.contains(&ident) {
        Some(Width::NonZeroSigned)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>, w: u32) -> u32 { hits.push(0); w }",
        )
        .is_none());
    }

    #[test]
    fn unsigned_param_fixes_to_bit_width() {
        let out = propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>, w: u32) -> u32 { hits.push(0); u32::BITS - w.leading_zeros() }",
        )
        .expect("matched");
        assert!(out.contains("w.bit_width()"), "wrong fix: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn signed_param_casts_unsigned_first() {
        let out = propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>, x: i32) -> u32 { hits.push(0); i32::BITS - x.leading_zeros() }",
        )
        .expect("matched");
        assert!(
            out.contains("x.cast_unsigned().bit_width()"),
            "wrong fix: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nonzero_unsigned_generic_param_appends_get() {
        let out = propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>, y: NonZero<u32>) -> u32 { hits.push(0); NonZero::<u32>::BITS - y.leading_zeros() }",
        )
        .expect("matched");
        assert!(out.contains("y.bit_width().get()"), "wrong fix: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nonzero_signed_alias_param_casts_and_gets() {
        let out = propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>, y: NonZeroI32) -> u32 { hits.push(0); NonZeroI32::BITS - y.leading_zeros() }",
        )
        .expect("matched");
        assert!(
            out.contains("y.cast_unsigned().bit_width().get()"),
            "wrong fix: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn mismatched_shape_fixes_by_receiver() {
        // A u8 receiver under a `u32::BITS` LHS — the mismatched-lint shape;
        // the fix is RECEIVER-driven (clippy's own suggestion,
        // probe-verified), and the second dispatch name routes to the same
        // shape.
        let out = propose_fix(
            "mismatched_bit_width_type",
            "fn f(hits: &mut Vec<u8>, w: u8) -> u32 { hits.push(0); u32::BITS - w.leading_zeros() }",
        )
        .expect("matched");
        assert!(out.contains("w.bit_width()"), "wrong fix: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn std_qualified_nonzero_lhs_passes() {
        let out = propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>, n: usize) -> u32 { hits.push(0); std::num::NonZero::<usize>::BITS - n.leading_zeros() }",
        )
        .expect("matched");
        assert!(out.contains("n.bit_width()"), "wrong fix: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn arithmetic_context_tail_preserved() {
        let out = propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>, w: u32) -> u32 { hits.push(0); let b = u32::BITS - w.leading_zeros(); b * 2 }",
        )
        .expect("matched");
        assert!(
            out.contains("let b = w.bit_width(); b * 2"),
            "tail not preserved: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn impl_method_param_proven() {
        let out = propose_fix(
            "manual_bit_width",
            "struct S; impl S { fn f(&self, hits: &mut Vec<u8>, x: i64) -> u32 { hits.push(0); i64::BITS - x.leading_zeros() } }",
        )
        .expect("matched");
        assert!(
            out.contains("x.cast_unsigned().bit_width()"),
            "wrong fix: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn literal_lhs_refuses() {
        // Probe-verified silent upstream — a literal LHS is not a BITS path.
        assert!(propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>, w: u32) -> u32 { hits.push(0); 1_000_000 - w.leading_zeros() }",
        )
        .is_none());
    }

    #[test]
    fn unproven_local_receiver_refuses() {
        // A let-bound receiver is invisible to the param-only derivation —
        // no proven type, no fix kind to pick (clippy can type it; we
        // refuse).
        assert!(propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>) -> u32 { hits.push(0); let w = 3u32; u32::BITS - w.leading_zeros() }",
        )
        .is_none());
    }

    #[test]
    fn float_receiver_refuses() {
        // Proven to a NON-integer type — no `bit_width()` exists there.
        assert!(propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>, g: f64) -> u32 { hits.push(0); f64::BITS - g.leading_zeros() }",
        )
        .is_none());
    }

    #[test]
    fn leading_zeros_with_args_refuses() {
        assert!(propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>, w: u32) -> u32 { hits.push(0); u32::BITS - w.leading_zeros(1) }",
        )
        .is_none());
    }

    #[test]
    fn chained_receiver_refuses() {
        // The receiver is a method call, not a bare proven ident — even
        // though the base `w` is proven.
        assert!(propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>, w: u32) -> u32 { hits.push(0); u32::BITS - w.clone().leading_zeros() }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>, w: u32) -> u32 { hits.push(0); u32::BITS /* keep */ - w.leading_zeros() }",
        )
        .is_none());
    }

    #[test]
    fn wrong_lhs_segment_refuses() {
        // `MAX`, not `BITS` — never our shape.
        assert!(propose_fix(
            "manual_bit_width",
            "fn f(hits: &mut Vec<u8>, w: u32) -> u32 { hits.push(0); u32::MAX - w.leading_zeros() }",
        )
        .is_none());
    }
}

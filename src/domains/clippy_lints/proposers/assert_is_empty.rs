//! `clippy::assert_is_empty` — `assert!(s.is_empty())` → `assert_eq!(s, "")`
//! (negated → `assert_ne!`, `debug_assert!` twins map to the `_eq`/`_ne`
//! debug forms) on a signature-proven receiver (Batch 230 / Plan 232 — NEW
//! in clippy 1.99 / PR #17149, pedantic group; probe-verified 2026-10-13
//! against clippy 1.99.0 (isolated toolchain); the fixed forms compile at
//! 1.98.1 too).
//!
//! ## Why the rewrite is pass-behavior identical (the FIXABLE bar)
//!
//! `assert_eq!(a, b)` is defined by std as exactly `assert!(*a == *b)`
//! (modulo the by-reference capture dance) — so the PASS path of the
//! rewrite is identical: same condition, same operands, same evaluation.
//! The PANIC MESSAGE differs — and that difference IS the lint's stated
//! purpose: on failure `assert_eq!` prints the asserted values
//! (`left: "x"`, `right: ""`) while `assert!` prints only the literal
//! message; printing the asserted value on failure is the improvement the
//! lint exists to make, not a behavior regression.
//!
//! The type-blindness leg (no type checker here): the receiver's family is
//! proven from the fn SIGNATURE — a plain never-rebound param whose
//! annotation is the CONSTRUCTION fact (`body_rebinds` soundness leg, the
//! `scalar_param_heads_over`/`slice_param_heads_over` posture) — so the
//! right-hand side is exactly that family's empty value: `""` for the
//! str/String family, `[]` for slices and slice views, `[] as [T; 0]` for
//! owned `Vec<T>` (the cast spells `T`, the one shape where a bare `[]`
//! under-infers). Every fixed form below was compile-verified at 1.99.0
//! and 1.98.1 (probe, 2026-10-13).
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - The macro path is EXACTLY `assert` or `debug_assert` — a plain
//!   single segment, no generic args (`assert_eq` is NOT our input; it is
//!   the OUTPUT; multi-segment/qualified spellings refuse).
//! - EXACTLY ONE macro argument — a custom message (`assert!(v, "msg")`,
//!   2+ args) never fires, matching the verified upstream silence.
//! - The single argument is `x.is_empty()` or `!x.is_empty()`: method
//!   exactly `is_empty`, ZERO args, no turbofish, receiver a BARE path
//!   ident. Method chains (`s.trim().is_empty()`), fields, indexes,
//!   calls, and macro-expanded receivers (`m!().is_empty()`) refuse.
//! - The receiver ident is PROVEN from the enclosing fn signature
//!   (`ItemFn` + `ImplItemFn`): plain `Pat::Ident`, no `ref`/subpattern,
//!   never rebound/assigned anywhere in the body (`body_rebinds` — the
//!   signature annotation is then a construction fact, not an inference).
//!   Locals (`let` bindings) and closure params are never proven.
//! - Proven families and their fixed forms: `&str` / `String` /
//!   `&String` → `assert_eq!(s, "")`; `&[T]` → `assert_eq!(s, [])`;
//!   `&Vec<T>` → `assert_eq!(s.as_slice(), [])`; owned `Vec<T>` →
//!   `assert_eq!(v, [] as [T; 0])` (`T` spelled via canonical token text).
//!   Classification is STRUCTURAL on the syn type node — the conservative
//!   reading of "exact source spelling": `&mut` anything, `&[T; N]`
//!   arrays (a `[T; N] == [T; 0]` compare would not compile),
//!   multi-segment paths (`std::vec::Vec<T>`), qualified paths, `HashMap`
//!   and every other type text → REFUSE.
//! - Negation flips `_eq` → `_ne`; the `debug_` prefix carries.
//! - A comment inside the replaced macro span refuses (only the
//!   receiver's own bytes survive verbatim in the splice).
//! - Macros nested inside OTHER macros' token streams are invisible to
//!   syn's visitor (opaque tokens, no descent) — conservative silence.
//!
//! House note: the arg extraction is `all_expr_args`, NOT
//! `macro_expr_args` — the assert CONDITION is the PRE-template operand
//! (family position `Second`), which `post_template_expr_args` skips, so
//! `macro_expr_args` returns an empty vec for every bare `assert!(cond)`
//! (dead for this lint's shape). `all_expr_args` includes the pre-template
//! operands — the same lane `match_bool` uses for `assert!(match ..)`.

use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::visit::Visit;
use syn::{Expr, ExprMacro, File, ImplItemFn, ItemFn, Macro, StmtMacro};

use super::helpers::{body_rebinds, byte_range, drops_interior_comment};
use super::uninlined_format_args::all_expr_args;
use super::{edit, src, SourceEdit};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        proven: BTreeMap::new(),
    };
    v.visit_file(file);
    v.edits
}

/// The signature-proven family of a param's type — decides the right-hand
/// side of the rewrite and whether the receiver is re-spelled.
enum ParamType {
    /// `&str` / `String` / `&String` — compare against `""`.
    Str,
    /// `&[T]` — compare against `[]`.
    Slice,
    /// `&Vec<T>` — re-spell the receiver `.as_slice()`, compare against
    /// `[]` (the probe-verified fixed form for the shared-Vec family).
    VecRef,
    /// Owned `Vec<T>` (element text spelled) — compare against
    /// `[] as [T; 0]` (the cast names `T`, the one shape where a bare
    /// `[]` under-infers).
    VecOwned(String),
}

impl ParamType {
    /// The replacement macro invocation for the whole assert span —
    /// `name!(lhs, rhs)` with the variant/prefix carried from the input.
    fn rewrite(&self, debug: bool, negated: bool, recv: &str) -> String {
        let name = match (debug, negated) {
            (false, false) => "assert_eq",
            (false, true) => "assert_ne",
            (true, false) => "debug_assert_eq",
            (true, true) => "debug_assert_ne",
        };
        let lhs = match self {
            Self::VecRef => format!("{recv}.as_slice()"),
            _ => recv.to_string(),
        };
        let rhs = match self {
            Self::Str => "\"\"".to_string(),
            Self::Slice | Self::VecRef => "[]".to_string(),
            Self::VecOwned(elem) => format!("[] as [{elem}; 0]"),
        };
        format!("{name}!({lhs}, {rhs})")
    }
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Signature-proven bindings of the enclosing fn (type family).
    proven: BTreeMap<String, ParamType>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        // Save/restore via a plain local — correct under arbitrary
        // nesting (an inner fn's params replace the outer map for the
        // inner body only).
        let saved = std::mem::take(&mut self.proven);
        self.proven = proven_params(f.sig.inputs.iter(), &f.block);
        syn::visit::visit_item_fn(self, f);
        self.proven = saved;
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = proven_params(f.sig.inputs.iter(), &f.block);
        syn::visit::visit_impl_item_fn(self, f);
        self.proven = saved;
    }

    fn visit_expr_macro(&mut self, m: &'ast ExprMacro) {
        syn::visit::visit_expr_macro(self, m);
        self.scan_macro(&m.mac);
    }

    /// Statement-position macros (`assert!(…);`) parse as `Stmt::Macro`,
    /// not `Expr::Macro` — both shapes are visited (the `match_bool`
    /// precedent).
    fn visit_stmt_macro(&mut self, m: &'ast StmtMacro) {
        syn::visit::visit_stmt_macro(self, m);
        self.scan_macro(&m.mac);
    }
}

impl Collector<'_> {
    fn scan_macro(&mut self, mac: &Macro) {
        if let Some(replacement) = self.try_rewrite(mac) {
            self.edits.push(edit(mac, replacement));
        }
    }

    /// The whole-macro rewrite when every guard passes — `None` refuses.
    fn try_rewrite(&self, mac: &Macro) -> Option<String> {
        let debug = plain_assert_variant(mac)?;
        let args = all_expr_args(mac)?;
        if args.len() != 1 {
            return None; // custom message / format args — upstream silence
        }
        let toks: TokenStream = args[0].toks.iter().cloned().collect();
        let expr = syn::parse2::<Expr>(toks).ok()?;
        let (negated, call) = match &expr {
            Expr::Unary(u) if matches!(u.op, syn::UnOp::Not(_)) => (true, &*u.expr),
            other => (false, other),
        };
        let Expr::MethodCall(m) = call else {
            return None;
        };
        if m.method != "is_empty" || !m.args.is_empty() || m.turbofish.is_some() {
            return None;
        }
        let ident = bare_ident(&m.receiver)?;
        let family = self.proven.get(ident.as_str())?;
        // The whole macro span is re-spelled; only the receiver's own
        // bytes survive verbatim — a comment anywhere else would drop.
        if drops_interior_comment(self.source, byte_range(mac), &[byte_range(&*m.receiver)]) {
            return None;
        }
        let recv = src(self.source, &*m.receiver)?;
        Some(family.rewrite(debug, negated, &recv))
    }
}

/// The signature-proven bindings of one fn: `{ident → type family}` for
/// every PLAIN `Pat::Ident` param whose annotation classifies (the
/// `slice_param_heads_over` loop shape) — with the shared `body_rebinds`
/// soundness leg: a never-rebound param keeps exactly the signature's
/// type at every use, which makes the family a CONSTRUCTION fact rather
/// than an inference. The `self` receiver is a `Receiver` arg, skipped by
/// the Typed-only scan.
fn proven_params<'i>(
    inputs: impl Iterator<Item = &'i syn::FnArg>,
    block: &syn::Block,
) -> BTreeMap<String, ParamType> {
    let mut out = BTreeMap::new();
    for arg in inputs {
        let syn::FnArg::Typed(p) = arg else { continue };
        let syn::Pat::Ident(pi) = &*p.pat else {
            continue;
        };
        if pi.by_ref.is_some() || pi.subpat.is_some() {
            continue;
        }
        let Some(family) = classify_param_type(&p.ty) else {
            continue;
        };
        if body_rebinds(block, &pi.ident.to_string()) {
            continue;
        }
        out.insert(pi.ident.to_string(), family);
    }
    out
}

/// Structural classification of a provable annotation into its rewrite
/// family — the conservative posture of `scalar_head_of`/`elem_head_of`:
/// every unusual shape (mut refs, arrays, multi-segment/qualified paths,
/// non-container types) refuses.
fn classify_param_type(ty: &syn::Type) -> Option<ParamType> {
    match ty {
        syn::Type::Reference(r) => {
            if r.mutability.is_some() {
                return None;
            }
            match &*r.elem {
                syn::Type::Slice(_) => Some(ParamType::Slice),
                syn::Type::Path(p) => {
                    let seg = sole_segment(p)?;
                    match seg.ident.to_string().as_str() {
                        "str" | "String" => Some(ParamType::Str),
                        "Vec" => {
                            // Shape check only — exactly one type argument
                            // must be present; the T text is not needed.
                            sole_type_argument(seg)?;
                            Some(ParamType::VecRef)
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        syn::Type::Path(p) => {
            let seg = sole_segment(p)?;
            match seg.ident.to_string().as_str() {
                "String" => Some(ParamType::Str),
                "Vec" => Some(ParamType::VecOwned(type_text(sole_type_argument(seg)?))),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The sole path segment of a plain path type — no qself, no leading
/// `::`, exactly one segment (`std::vec::Vec<T>` and qualified spellings
/// refuse).
fn sole_segment(p: &syn::TypePath) -> Option<&syn::PathSegment> {
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    p.path.segments.first()
}

/// The single type argument of a generic segment (`Vec<T>` → `T`) —
/// exactly one angle-bracketed argument and it is a type; `Vec<>`,
/// `Vec<T, U>` and lifetime-only shapes refuse.
fn sole_type_argument(seg: &syn::PathSegment) -> Option<&syn::Type> {
    let syn::PathArguments::AngleBracketed(ab) = &seg.arguments else {
        return None;
    };
    if ab.args.len() != 1 {
        return None;
    }
    match ab.args.first()? {
        syn::GenericArgument::Type(t) => Some(t),
        _ => None,
    }
}

/// Canonical token text of a type (the owned-`Vec` cast's element
/// spelling) — a bare ident prints clean; a nested generic prints with
/// token spacing, which is still-valid Rust at the cast position.
fn type_text(t: &syn::Type) -> String {
    t.to_token_stream().to_string()
}

/// The bare path ident of a receiver expression — `None` for every other
/// shape (chain, field, index, call, macro, multi-segment path).
fn bare_ident(e: &Expr) -> Option<String> {
    let Expr::Path(p) = e else { return None };
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    if !p.path.segments[0].arguments.is_none() {
        return None;
    }
    Some(p.path.segments[0].ident.to_string())
}

/// `Some(debug)` when the macro path is exactly `assert` / `debug_assert`
/// (a plain single segment, no generic args); `None` for every other
/// macro — `assert_eq` inputs are not ours, multi-segment spellings
/// refuse.
fn plain_assert_variant(mac: &Macro) -> Option<bool> {
    let p = &mac.path;
    if p.leading_colon.is_some() || p.segments.len() != 1 {
        return None;
    }
    let seg = p.segments.first()?;
    if !seg.arguments.is_none() {
        return None;
    }
    match seg.ident.to_string().as_str() {
        "assert" => Some(false),
        "debug_assert" => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const RULE: &str = "assert_is_empty";

    #[test]
    fn seed_is_clean() {
        // Clean code — the `if` condition is NOT an assert context
        // (upstream silence for non-assert contexts), and no macro fires.
        assert!(propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u8>, v: Vec<u32>) -> bool { hits.push(0); if v.is_empty() { return true; } false }",
        )
        .is_none());
    }

    #[test]
    fn str_family_fires() {
        // `&str`, `String` and `&String` all take the `""` form.
        let out = propose_fix(
            RULE,
            "fn f(a: &str, b: String, c: &String) { assert!(a.is_empty()); assert!(b.is_empty()); assert!(c.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(a, \"\")"),
            "a not rewritten: {out}"
        );
        assert!(
            out.contains("assert_eq!(b, \"\")"),
            "b not rewritten: {out}"
        );
        assert!(
            out.contains("assert_eq!(c, \"\")"),
            "c not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn slice_param_fires() {
        let out = propose_fix(RULE, "fn f(s: &[u32]) { assert!(s.is_empty()); }").expect("matched");
        assert!(
            out.contains("assert_eq!(s, [])"),
            "slice not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn vec_ref_param_fires() {
        // `&Vec<T>` re-spells the slice view — the probe-verified form.
        let out =
            propose_fix(RULE, "fn f(v: &Vec<u32>) { assert!(v.is_empty()); }").expect("matched");
        assert!(
            out.contains("assert_eq!(v.as_slice(), [])"),
            "vec-ref not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn owned_vec_param_fires() {
        // Owned `Vec<T>` spells the element type in the cast.
        let out =
            propose_fix(RULE, "fn f(v: Vec<u32>) { assert!(v.is_empty()); }").expect("matched");
        assert!(
            out.contains("assert_eq!(v, [] as [u32; 0])"),
            "owned vec not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn negated_fires_assert_ne() {
        let out = propose_fix(RULE, "fn f(s: &str) { assert!(!s.is_empty()); }").expect("matched");
        assert!(
            out.contains("assert_ne!(s, \"\")"),
            "negated not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn debug_assert_variants_fire() {
        // The `debug_` prefix carries across both polarities.
        let out = propose_fix(
            RULE,
            "fn f(s: &str) { debug_assert!(s.is_empty()); debug_assert!(!s.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("debug_assert_eq!(s, \"\")"),
            "debug assert not rewritten: {out}"
        );
        assert!(
            out.contains("debug_assert_ne!(s, \"\")"),
            "debug negated not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_in_bigger_fn_fires() {
        let out = propose_fix(
            RULE,
            "fn f(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(1); if s.is_empty() { assert!(s.is_empty()); } hits.len() }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(1);"), "context lost: {out}");
        assert!(
            out.contains("assert_eq!(s, \"\")"),
            "nested assert not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn impl_method_param_fires() {
        let out = propose_fix(
            RULE,
            "struct S; impl S { fn f(&self, s: &str) { assert!(s.is_empty()); } }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(s, \"\")"),
            "impl-fn assert not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn custom_message_refuses() {
        // Upstream silence: 2+ macro args carry a message — never ours.
        assert!(propose_fix(
            RULE,
            "fn f(v: Vec<u32>) { assert!(v.is_empty(), \"not empty: {v:?}\"); }",
        )
        .is_none());
    }

    #[test]
    fn method_chain_receiver_refuses() {
        // `s.trim().is_empty()` — the receiver is not a bare ident.
        assert!(propose_fix(RULE, "fn f(s: &str) { assert!(s.trim().is_empty()); }",).is_none());
    }

    #[test]
    fn unproven_local_refuses() {
        // A `let` local is never in the signature-proven map.
        assert!(propose_fix(
            RULE,
            "fn f() { let v: Vec<u32> = Vec::new(); assert!(v.is_empty()); }",
        )
        .is_none());
    }

    #[test]
    fn mut_vec_ref_refuses() {
        // `&mut Vec<u32>` — the mut reference refuses (conservative).
        assert!(propose_fix(RULE, "fn f(v: &mut Vec<u32>) { assert!(v.is_empty()); }",).is_none());
    }

    #[test]
    fn macro_expanded_receiver_refuses() {
        // A macro-expansion receiver is not a bare proven ident.
        assert!(propose_fix(RULE, "fn f() { assert!(mk!().is_empty()); }",).is_none());
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(
            propose_fix(RULE, "fn f(s: &str) { assert!(s /* keep */ .is_empty()); }",).is_none()
        );
    }

    #[test]
    fn hashmap_param_refuses() {
        // `&HashMap<K, V>` is not in the proven family map.
        assert!(propose_fix(
            RULE,
            "use std::collections::HashMap; fn f(m: &HashMap<u32, u32>) { assert!(m.is_empty()); }",
        )
        .is_none());
    }
}

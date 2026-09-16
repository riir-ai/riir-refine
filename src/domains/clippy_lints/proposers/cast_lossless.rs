//! `clippy::cast_lossless` — `x as u64` → `u64::from(x)` where the cast is
//! provably widening (Batch 150; the staged pedantic trio's VERIFY-shaped
//! member — the one `cast_*` family lint with a mechanical rewrite, since
//! the other four are *warnings about* a cast rather than rewrites of one).
//!
//! ## Probe matrix (real clippy 0.1.98, `-W clippy::pedantic`)
//!
//! FIRES — every primitive widening cast, INCLUDING the sign change
//! (`u32 as i64` → `i64::from(u32v)` — `From<u32> for i64` exists), and
//! float widening (`f32 as f64`). Every suggestion is `MachineApplicable`
//! and single-child. SILENT — narrowing casts, same-type casts (that is
//! `unnecessary_cast`), and non-numeric casts.
//!
//! Tier: VERIFY via the SAFE-LET gate. Real clippy resolves the operand's
//! type with typeck; the parse-only proposer can only follow an EXPLICIT
//! type annotation. The gate fires on exactly two shapes:
//!
//! 1. `let x: u8 = …; let y = x as u64;` — a unique `let`-binding whose
//!    pattern carries the type annotation;
//! 2. `fn f(x: u8) -> u64 { x as u64 }` — a uniquely-named fn parameter.
//!
//! In both, the source type is written down, so the rewrite is emitted
//! only when `From<Src> for Dst` is one of std's primitive impls (an
//! explicit table — `From` is infallible by definition, so the rewrite is
//! value-identical to the `as` cast whenever it compiles). Everything
//! else refuses; the P21 oracle lane consumes real clippy's
//! `MachineApplicable` suggestion for the residue (fields, generics,
//! complex operands). A wrong fire is E0277/E0599 — check-visible, the
//! compile gate auto-reverts.
//!
//! Divergence (documented): shadowing is resolved conservatively — a
//! SECOND `let` binding of the same name anywhere in the file refuses
//! (the annotation-pinned type could belong to a different binding than
//! the cast site reads).

use super::{src, SourceEdit};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprCast, File, FnArg, ItemFn, Local, Pat};

/// std's `From` implementations between primitive numerics (the
/// lossless/widening subset — exactly what `cast_lossless` fires on).
fn from_impl_exists(src: &str, dst: &str) -> bool {
    match src {
        "u8" => matches!(
            dst,
            "u16" | "u32" | "u64" | "u128" | "usize" | "i16" | "i32" | "i64" | "i128" | "isize"
                | "f32" | "f64"
        ),
        "u16" => matches!(
            dst,
            "u32" | "u64" | "u128" | "usize" | "i32" | "i64" | "i128" | "f32" | "f64"
        ),
        "u32" => matches!(dst, "u64" | "u128" | "i64" | "i128" | "f64"),
        "u64" => matches!(dst, "u128"),
        "i8" => matches!(
            dst,
            "i16" | "i32" | "i64" | "i128" | "isize" | "f32" | "f64"
        ),
        "i16" => matches!(dst, "i32" | "i64" | "i128" | "f32" | "f64"),
        "i32" => matches!(dst, "i64" | "i128" | "f64"),
        "i64" | "isize" => matches!(dst, "i128"),
        "f32" => matches!(dst, "f64"),
        "usize" => matches!(dst, "u128" | "i128"),
        // u128/i128/f64 are maximal — no widening target exists.
        _ => false,
    }
}

fn is_prim(name: &str) -> bool {
    matches!(
        name,
        "i8" | "u8"
            | "i16"
            | "u16"
            | "i32"
            | "u32"
            | "i64"
            | "u64"
            | "i128"
            | "u128"
            | "isize"
            | "usize"
            | "f32"
            | "f64"
    )
}

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        bindings: annotated_bindings(file),
        params: annotated_params(file),
        all_lets: all_let_names(file),
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    /// Unique explicit `let x: T = …` bindings (name → type).
    bindings: Vec<(String, String)>,
    /// Explicit fn params `x: T` (name → type), per fn.
    params: Vec<(String, String)>,
    /// EVERY `let <name>` in the file (the shadow guard's population).
    all_lets: Vec<String>,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_cast(&mut self, e: &'ast ExprCast) {
        syn::visit::visit_expr_cast(self, e);
        let Expr::Path(p) = &*e.expr else {
            return;
        };
        let Some(ident) = p.path.get_ident() else {
            return;
        };
        let ident = ident.to_string();
        let Some(dst) = prim_ty_path(&e.ty) else {
            return;
        };
        // The operand's type: the unique annotated binding or param — and
        // the shadow guard: ANY second `let` of the same name refuses (the
        // annotation-pinned type could belong to a different binding than
        // the cast site reads).
        let lets_of_name = self.all_lets.iter().filter(|n| n.as_str() == ident).count();
        if lets_of_name > 1 {
            return;
        }
        let mut annotated: Vec<&String> = self
            .bindings
            .iter()
            .chain(&self.params)
            .filter(|(n, _)| *n == ident)
            .map(|(_, t)| t)
            .collect();
        if annotated.len() != 1 {
            return; // unknown / unannotated — refuse
        }
        let src_ty = annotated.swap_remove(0).clone();
        if !from_impl_exists(&src_ty, &dst) {
            return;
        }
        let Some(operand) = src(self.source, p) else {
            return;
        };
        self.edits.push(SourceEdit {
            start: e.span().byte_range().start,
            end: e.span().byte_range().end,
            replacement: format!("{dst}::from({operand})"),
        });
    }
}

/// `u8` → `Some("u8")`; anything else (generic args, qualified paths,
/// references) → `None`.
fn prim_ty_path(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(p) = ty else {
        return None;
    };
    let ident = p.path.get_ident()?;
    let name = ident.to_string();
    if is_prim(&name) {
        Some(name)
    } else {
        None
    }
}

/// Every `let x: T = …` binding with an explicit primitive-type
/// annotation (the safe-let class — P15's receiver-kind precedent).
fn annotated_bindings(file: &File) -> Vec<(String, String)> {
    struct Bindings(Vec<(String, String)>);
    impl<'ast> Visit<'ast> for Bindings {
        fn visit_local(&mut self, l: &'ast Local) {
            if let Pat::Type(pt) = &l.pat {
                if let Pat::Ident(pi) = &*pt.pat {
                    if !pi.ident.to_string().starts_with("r#") {
                        if let Some(t) = prim_ty_path(&pt.ty) {
                            self.0.push((pi.ident.to_string(), t));
                        }
                    }
                }
            }
            syn::visit::visit_local(self, l);
        }
    }
    let mut b = Bindings(Vec::new());
    b.visit_file(file);
    b.0
}

/// Every `let <name>` binding in the file, top-level ident patterns only
/// (tuple/struct patterns bind multiple names — the conservative scan
/// skips them; a missed shadow is the compile gate's revert, an
/// over-cautious refusal only shrinks the fired set).
fn all_let_names(file: &File) -> Vec<String> {
    struct Lets(Vec<String>);
    impl<'ast> Visit<'ast> for Lets {
        fn visit_local(&mut self, l: &'ast Local) {
            // Count ANNOTATED-pattern locals too (`let x: u8 = ..` parses as
            // Pat::Type wrapping Pat::Ident) — the shadow guard needs EVERY
            // binding of the name, not just the bare-ident ones.
            match &l.pat {
                Pat::Ident(pi) => self.0.push(pi.ident.to_string()),
                Pat::Type(pt) => {
                    if let Pat::Ident(pi) = &*pt.pat {
                        self.0.push(pi.ident.to_string());
                    }
                }
                _ => {}
            }
            syn::visit::visit_local(self, l);
        }
    }
    let mut v = Lets(Vec::new());
    v.visit_file(file);
    v.0
}

/// Every fn parameter with an explicit primitive-type annotation.
fn annotated_params(file: &File) -> Vec<(String, String)> {
    struct Params(Vec<(String, String)>);
    impl<'ast> Visit<'ast> for Params {
        fn visit_item_fn(&mut self, f: &'ast ItemFn) {
            for arg in &f.sig.inputs {
                if let FnArg::Typed(pt) = arg {
                    if let Pat::Ident(pi) = &*pt.pat {
                        if !pi.ident.to_string().starts_with("r#") {
                            if let Some(t) = prim_ty_path(&pt.ty) {
                                self.0.push((pi.ident.to_string(), t));
                            }
                        }
                    }
                }
            }
            syn::visit::visit_item_fn(self, f);
        }
    }
    let mut p = Params(Vec::new());
    p.visit_file(file);
    p.0
}

#[cfg(test)]
mod tests {

    fn fix(code: &str) -> Option<String> {
        super::super::propose_fix("cast_lossless", code)
    }

    #[test]
    fn fires_on_annotated_binding_and_param_shapes() {
        for (buggy, fixed) in [
            (
                "fn widen() -> u64 { let x: u8 = 5; x as u64 }",
                "fn widen() -> u64 { let x: u8 = 5; u64::from(x) }",
            ),
            (
                "fn widen(x: u8) -> u64 { x as u64 }",
                "fn widen(x: u8) -> u64 { u64::from(x) }",
            ),
            (
                "fn widen(x: i32) -> i64 { x as i64 }",
                "fn widen(x: i32) -> i64 { i64::from(x) }",
            ),
            (
                // The probe's sign-change shape — u32 as i64 FIRES.
                "fn widen(x: u32) -> i64 { x as i64 }",
                "fn widen(x: u32) -> i64 { i64::from(x) }",
            ),
            (
                "fn widen() -> f64 { let h: f32 = 1.0; h as f64 }",
                "fn widen() -> f64 { let h: f32 = 1.0; f64::from(h) }",
            ),
        ] {
            assert_eq!(fix(buggy).as_deref(), Some(fixed), "case: {buggy}");
        }
    }

    #[test]
    fn refuses_unknown_narrowing_and_shadowed_shapes() {
        for code in [
            // Unannotated operand — the type is invisible parse-only.
            "fn widen(x: u8) -> u64 { let y = 5; y as u64 }",
            // Narrowing — no From impl.
            "fn narrow(x: u64) -> u8 { let z: u64 = x; z as u8 }",
            // Sign-reinterpret, not lossless — u64 as i64 has no From.
            "fn flip(x: u64) -> i64 { let z: u64 = x; z as i64 }",
            // Shadowed binding — the annotation may not belong to this site.
            "fn s() -> u64 { let x: u8 = 5; let x: u8 = 6; x as u64 }",
            // Second binding of the same name (conservative refusal).
            "fn s() -> u64 { let x: u8 = 5; { let x = 9; } x as u64 }",
            // Same-type cast — unnecessary_cast's territory, no widening.
            "fn same(x: u8) -> u8 { let z: u8 = x; z as u8 }",
            // Field/method operands — the oracle lane's residue.
            "struct S { x: u8 }\nfn f(s: S) -> u64 { s.x as u64 }",
        ] {
            assert_eq!(fix(code), None, "must refuse: {code}");
        }
    }
}

//! `clippy::approx_constant` — a float literal whose value is BIT-IDENTICAL
//! to a known `std::fXX::consts` constant → the fully-qualified const path.
//!
//! Probe-mapped trigger surface (clippy 1.99.0, 2026-11-10 — the P111
//! intake, the riir-shader `clipping.rs:62` golden-ratio instance, and the
//! P94 probe matrix re-verified): the lint fires DENY-BY-DEFAULT in every
//! literal position (const items, let bindings, call arguments), and its
//! diagnostics are HELP-ONLY (probed: zero suggestion children — the
//! "consider using the constant directly" child carries no replacement),
//! so the P21 oracle lane cannot heal the class; this corpus proposer is
//! the only fix path.
//!
//! The rewrite is value-preserving ONLY when the literal parses (at its
//! own type) to the SAME BITS as the constant — clippy's surface is
//! tolerance-based (the truncated `3.14159265358979` PI shape fires
//! upstream, probed) while ours is bit-identity, so this lane is a STRICT
//! SUBSET of real clippy: the truncated-literal class is the documented
//! divergence (the manual fix is sanctioned there; a silent value change
//! is never acceptable in a mechanical healer).
//!
//! Fires on:
//! - a SUFFIXED literal (`1.618033988749895_f64`, `1.61803398875_f32`) —
//!   the suffix pins the type and the value parses directly AT that type
//!   (no double rounding), so the table choice is parse-provable at any
//!   expression position;
//! - an UNSUFFIXED literal whose enclosing position DECLARES `f64`: a
//!   const/static item or a let binding with a `: f64` annotation
//!   (`const G: f64 = 1.618…;` — the intake's own live shape). f32-typed
//!   unsuffixed positions refuse: the file-local lane cannot reproduce
//!   rustc's direct-at-f32 rounding (an f64 parse + cast is a double
//!   rounding — negligible for the known constants but nonzero, and the
//!   class is rare).
//!
//! Refusals (disclosed divergences — real clippy is type-aware):
//! - negated literals (`-0.6931471805599453` — upstream FIRES the `LN_2`
//!   shape, probed; the rewrite would need `-(std::…)` and the negation's
//!   precedence interactions stay out of scope);
//! - unsuffixed literals in untyped positions (a bare un-annotated `let`,
//!   call arguments, comparisons) — f64-vs-f32 inference is typeck
//!   territory and an f32-inferred context given an f64 path is an E0308;
//! - truncated values (the bit-identity gate above);
//! - the `#![no_std]` question: the emitted path is fully-qualified
//!   `std::fXX::consts::NAME`; a `no_std` crate is invisible to the
//!   file-local proposer and the compile gate reverts the edit there (the
//!   documented residual). Float literals inside macro calls never reach
//!   this visitor (syn does not parse macro interiors), so the macro-arg
//!   class is structurally out of reach, not a guard.

use super::SourceEdit;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{ExprLit, File, ItemConst, ItemStatic, Lit, LitFloat, Local, Pat, Type};

/// One known constant: the `std::fXX::consts` name plus the canonical f64
/// value — sourced from the std consts THEMSELVES (never re-typed digits:
/// a transcription slip would fail the bit pins, and the `approx_constant` /
/// `unreadable_literal` lints this crate denies would fire on our own
/// table). The f32 comparison derives `(value as f32).to_bits()` at match
/// time; the module tests pin EVERY entry's derived f64/f32 bits against
/// the actual `std::consts` items, so a std-side value change or a
/// double-rounding discrepancy fails the batch loudly.
struct KnownConst {
    name: &'static str,
    value: f64,
}

/// The constant set real clippy 1.99's `approx_constant` carries (probed
/// 2026-11-10, fixture matrix in `.raw/b348_probe`, removed at close):
/// `SQRT_3` is deliberately ABSENT — std carries it but clippy 1.99 does
/// not fire it, and a proposer const upstream is silent on would rewrite
/// code the target crate's own clippy calls clean (the Issue-135
/// inversion).
const KNOWN_CONSTS: &[KnownConst] = &[
    KnownConst {
        name: "E",
        value: std::f64::consts::E,
    },
    KnownConst {
        name: "EULER_GAMMA",
        value: std::f64::consts::EULER_GAMMA,
    },
    KnownConst {
        name: "FRAC_1_PI",
        value: std::f64::consts::FRAC_1_PI,
    },
    KnownConst {
        name: "FRAC_1_SQRT_2",
        value: std::f64::consts::FRAC_1_SQRT_2,
    },
    KnownConst {
        name: "FRAC_2_PI",
        value: std::f64::consts::FRAC_2_PI,
    },
    KnownConst {
        name: "FRAC_2_SQRT_PI",
        value: std::f64::consts::FRAC_2_SQRT_PI,
    },
    KnownConst {
        name: "FRAC_PI_2",
        value: std::f64::consts::FRAC_PI_2,
    },
    KnownConst {
        name: "FRAC_PI_3",
        value: std::f64::consts::FRAC_PI_3,
    },
    KnownConst {
        name: "FRAC_PI_4",
        value: std::f64::consts::FRAC_PI_4,
    },
    KnownConst {
        name: "FRAC_PI_6",
        value: std::f64::consts::FRAC_PI_6,
    },
    KnownConst {
        name: "FRAC_PI_8",
        value: std::f64::consts::FRAC_PI_8,
    },
    KnownConst {
        name: "GOLDEN_RATIO",
        value: std::f64::consts::GOLDEN_RATIO,
    },
    KnownConst {
        name: "LN_10",
        value: std::f64::consts::LN_10,
    },
    KnownConst {
        name: "LN_2",
        value: std::f64::consts::LN_2,
    },
    KnownConst {
        name: "LOG10_2",
        value: std::f64::consts::LOG10_2,
    },
    KnownConst {
        name: "LOG10_E",
        value: std::f64::consts::LOG10_E,
    },
    KnownConst {
        name: "LOG2_10",
        value: std::f64::consts::LOG2_10,
    },
    KnownConst {
        name: "LOG2_E",
        value: std::f64::consts::LOG2_E,
    },
    KnownConst {
        name: "PI",
        value: std::f64::consts::PI,
    },
    KnownConst {
        name: "SQRT_2",
        value: std::f64::consts::SQRT_2,
    },
    KnownConst {
        name: "TAU",
        value: std::f64::consts::TAU,
    },
];

/// The literal's parsed value + a type (`"f64"`/`"f32"`) → the const name
/// whose bits it reproduces at that type, if any. The f32 arm compares
/// the DIRECTLY-parsed f32 (`base10_parse::<f32>` — no double rounding)
/// against the table value's own f64→f32 cast.
fn match_const(lit: &LitFloat, is_f32: bool) -> Option<&'static str> {
    if is_f32 {
        let parsed: f32 = lit.base10_parse().ok()?;
        // The truncation IS the comparison: the table value narrowed to
        // f32 must reproduce the exact bits the target crate's compiler
        // would give the constant (pinned per-entry against the real
        // `std::f32::consts` items by `table_matches_std_consts_bit_for_bit`).
        #[allow(clippy::cast_possible_truncation)]
        let narrowed = |c: &KnownConst| c.value as f32;
        KNOWN_CONSTS
            .iter()
            .find(|c| parsed.to_bits() == narrowed(c).to_bits())
            .map(|c| c.name)
    } else {
        let parsed: f64 = lit.base10_parse().ok()?;
        KNOWN_CONSTS
            .iter()
            .find(|c| parsed.to_bits() == c.value.to_bits())
            .map(|c| c.name)
    }
}

/// `f64` | `f32` | None — a `: f64`/`: f32` annotation on a const/static
/// item or a let pattern, else None (a bare ident, a generic, a path type
/// all read None).
fn declared_float_type(ty: &Type) -> Option<&'static str> {
    let Type::Path(tp) = ty else { return None };
    if tp.qself.is_some() || tp.path.segments.len() != 1 {
        return None;
    }
    let seg = tp.path.segments.last()?;
    if !seg.arguments.is_none() {
        return None;
    }
    match seg.ident.to_string().as_str() {
        "f64" => Some("f64"),
        "f32" => Some("f32"),
        _ => None,
    }
}

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let _ = source; // whole-token rewrite — the lit span is the edit span.
    let mut v = Collector {
        edits: Vec::new(),
        declared: None,
    };
    v.visit_file(file);
    v.edits
}

struct Collector {
    edits: Vec<SourceEdit>,
    /// The innermost declared f64/f32 annotation (const/static item or
    /// let pattern), or None outside one.
    declared: Option<&'static str>,
}

impl Collector {
    fn try_lit(&mut self, el: &ExprLit) {
        let Lit::Float(f) = &el.lit else { return };
        let name = match f.suffix() {
            "f64" => match_const(f, false).map(|n| (n, "f64")),
            "f32" => match_const(f, true).map(|n| (n, "f32")),
            // Unsuffixed: only inside a declared `: f64` annotation (the
            // f32-annotated unsuffixed class refuses — the double-rounding
            // divergence named in the module docs).
            "" => match self.declared {
                Some("f64") => match_const(f, false).map(|n| (n, "f64")),
                _ => None,
            },
            _ => None,
        };
        let Some((name, ty)) = name else { return };
        self.edits.push(SourceEdit {
            start: el.span().byte_range().start,
            end: el.span().byte_range().end,
            replacement: format!("std::{ty}::consts::{name}"),
        });
    }
}

/// The unsuffixed arm keys on the innermost annotation; negated literals
/// refuse by construction (the literal sits under `Expr::Unary(Neg)`, and
/// only `Expr::Lit` reaches [`Collector::try_lit`]).
impl<'ast> Visit<'ast> for Collector {
    fn visit_expr_lit(&mut self, el: &'ast ExprLit) {
        self.try_lit(el);
        syn::visit::visit_expr_lit(self, el);
    }

    fn visit_expr_unary(&mut self, e: &'ast syn::ExprUnary) {
        // A negated float literal: upstream fires (probed), this lane
        // refuses — `-(std::f64::consts::LN_2)` needs the parenthesization
        // call, out of scope for v1.
        if matches!(e.op, syn::UnOp::Neg(_)) {
            return;
        }
        syn::visit::visit_expr_unary(self, e);
    }

    fn visit_item_const(&mut self, i: &'ast ItemConst) {
        let prev = self.declared;
        // The INNERMOST declaration wins — nested items always override
        // (a `let x: f64` inside an f32 const item annotates its own
        // literal, not the outer item's).
        self.declared = declared_float_type(&i.ty).or(prev);
        syn::visit::visit_item_const(self, i);
        self.declared = prev;
    }

    fn visit_item_static(&mut self, i: &'ast ItemStatic) {
        let prev = self.declared;
        self.declared = declared_float_type(&i.ty).or(prev);
        syn::visit::visit_item_static(self, i);
        self.declared = prev;
    }

    fn visit_local(&mut self, l: &'ast Local) {
        let prev = self.declared;
        // The annotation lives in `Pat::Type` (`let x: f64 = …`); a bare
        // ident pattern carries none. `or(prev)`: an un-annotated let
        // KEEPS the enclosing annotation (the const-item literal above a
        // plain statement still sees the item's declared type).
        let ty = match &l.pat {
            Pat::Type(pt) => declared_float_type(&pt.ty),
            _ => None,
        };
        self.declared = ty.or(prev);
        syn::visit::visit_local(self, l);
        self.declared = prev;
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    /// Every table entry's f64 bits AND derived f32 bits must equal the
    /// actual std constants — a wrong digit or a double-rounding
    /// discrepancy fails here, not in a consumer tree.
    #[test]
    fn table_matches_std_consts_bit_for_bit() {
        const TABLE: &[(&str, f64)] = &[
            ("E", std::f64::consts::E),
            ("EULER_GAMMA", std::f64::consts::EULER_GAMMA),
            ("FRAC_1_PI", std::f64::consts::FRAC_1_PI),
            ("FRAC_1_SQRT_2", std::f64::consts::FRAC_1_SQRT_2),
            ("FRAC_2_PI", std::f64::consts::FRAC_2_PI),
            ("FRAC_2_SQRT_PI", std::f64::consts::FRAC_2_SQRT_PI),
            ("FRAC_PI_2", std::f64::consts::FRAC_PI_2),
            ("FRAC_PI_3", std::f64::consts::FRAC_PI_3),
            ("FRAC_PI_4", std::f64::consts::FRAC_PI_4),
            ("FRAC_PI_6", std::f64::consts::FRAC_PI_6),
            ("FRAC_PI_8", std::f64::consts::FRAC_PI_8),
            ("GOLDEN_RATIO", std::f64::consts::GOLDEN_RATIO),
            ("LN_10", std::f64::consts::LN_10),
            ("LN_2", std::f64::consts::LN_2),
            ("LOG10_2", std::f64::consts::LOG10_2),
            ("LOG10_E", std::f64::consts::LOG10_E),
            ("LOG2_10", std::f64::consts::LOG2_10),
            ("LOG2_E", std::f64::consts::LOG2_E),
            ("PI", std::f64::consts::PI),
            ("SQRT_2", std::f64::consts::SQRT_2),
            ("TAU", std::f64::consts::TAU),
        ];
        assert_eq!(super::KNOWN_CONSTS.len(), TABLE.len(), "table size drift");
        for (name, std_value) in TABLE {
            let entry = super::KNOWN_CONSTS
                .iter()
                .find(|c| c.name == *name)
                .unwrap_or_else(|| panic!("{name} missing from KNOWN_CONSTS"));
            assert_eq!(
                entry.value.to_bits(),
                std_value.to_bits(),
                "{name} f64 digits wrong"
            );
            assert_eq!(
                (entry.value as f32).to_bits(),
                (*std_value as f32).to_bits(),
                "{name} f32 derivation wrong"
            );
        }
    }

    // ── the fire arms ──

    #[test]
    fn unsuffixed_typed_const_item_rewrites() {
        // The intake's own live shape (riir-shader clipping.rs:62).
        let out = propose_fix(
            "approx_constant",
            "pub const GOLDEN: f64 = 1.618033988749895;",
        )
        .expect("matched");
        assert!(out.contains("std::f64::consts::GOLDEN_RATIO"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn suffixed_f64_rewrites_anywhere() {
        let out = propose_fix(
            "approx_constant",
            "fn probe() -> f64 { 3.141592653589793_f64 }",
        )
        .expect("matched");
        assert!(out.contains("std::f64::consts::PI"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn suffixed_f32_rewrites() {
        let out = propose_fix("approx_constant", "pub const G: f32 = 1.61803398875_f32;")
            .expect("matched");
        assert!(out.contains("std::f32::consts::GOLDEN_RATIO"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn annotated_let_rewrites() {
        let out = propose_fix(
            "approx_constant",
            "fn probe() -> f64 { let g: f64 = 1.618033988749895; g }",
        )
        .expect("matched");
        assert!(out.contains("std::f64::consts::GOLDEN_RATIO"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn e_notation_and_underscore_digits_rewrite() {
        let out = propose_fix(
            "approx_constant",
            "pub const T: f64 = 6.283_185_307_179_586e0;",
        )
        .expect("matched");
        assert!(out.contains("std::f64::consts::TAU"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn f32_suffixed_matches_f32_bits_not_f64_digits() {
        // The f64 golden digits carry an f32 suffix: parsed AT f32 they
        // round to the same f32 bits as the f32 constant (probed: clippy
        // 1.99 fires this shape as f32 GOLDEN_RATIO).
        let out = propose_fix(
            "approx_constant",
            "pub const G: f32 = 1.618033988749895_f32;",
        )
        .expect("matched");
        assert!(out.contains("std::f32::consts::GOLDEN_RATIO"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    // ── the refusal arms ──

    #[test]
    fn truncated_literal_refuses() {
        // clippy's tolerance fires this shape; the bit-identity gate does
        // not — a silent value change is never acceptable.
        let out = propose_fix("approx_constant", "pub const P: f64 = 3.14159265358979;");
        assert!(out.is_none(), "truncated literal fired: {out:?}");
    }

    #[test]
    fn negated_literal_refuses() {
        let out = propose_fix("approx_constant", "pub const N: f64 = -0.6931471805599453;");
        assert!(out.is_none(), "negated literal fired: {out:?}");
    }

    #[test]
    fn unsuffixed_untyped_let_refuses() {
        // Inference is typeck territory (probed: clippy fires this shape
        // as f64 via its typeck — the f64-vs-f32 call is invisible here).
        let out = propose_fix(
            "approx_constant",
            "fn probe() -> f64 { let g = 1.618033988749895; g }",
        );
        assert!(out.is_none(), "untyped let fired: {out:?}");
    }

    #[test]
    fn unsuffixed_f32_annotated_refuses() {
        // The double-rounding class: an f64 parse + cast cannot reproduce
        // rustc's direct-at-f32 rounding guarantee.
        let out = propose_fix("approx_constant", "pub const G: f32 = 1.618033988749895;");
        assert!(out.is_none(), "f32-annotated unsuffixed fired: {out:?}");
    }

    #[test]
    fn unsuffixed_untyped_const_refuses() {
        let out = propose_fix("approx_constant", "pub const G = 1.618033988749895;");
        assert!(out.is_none(), "untyped const fired: {out:?}");
    }

    #[test]
    fn non_const_float_stays_silent() {
        let out = propose_fix("approx_constant", "pub const X: f64 = 2.0;");
        assert!(out.is_none(), "non-const float fired: {out:?}");
    }

    #[test]
    fn nested_annotation_innermost_wins() {
        // The inner let's f64 annotation governs its own literal even
        // inside an f32 const item.
        let out = propose_fix(
            "approx_constant",
            "pub const G: f32 = { let inner: f64 = 1.618033988749895; inner as f32 };",
        )
        .expect("matched");
        assert!(out.contains("std::f64::consts::GOLDEN_RATIO"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn untouched_bytes_survive() {
        let src =
            "// header\npub const GOLDEN: f64 = 1.618033988749895; // keep\nfn other() -> u8 { 1 }";
        let out = propose_fix("approx_constant", src).expect("matched");
        assert!(out.contains("// header\n"), "{out}");
        assert!(out.contains("// keep"), "{out}");
        assert!(out.contains("fn other() -> u8 { 1 }"), "{out}");
        assert_eq!(src.lines().count(), out.lines().count(), "{out}");
    }
}

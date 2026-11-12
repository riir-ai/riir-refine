//! `user-impl-derived-property-validate` — SUGGEST-ONLY detector
//! (Batch 353, the FORGE Lab std memory-safety post distill; Research 397 /
//! Plan 363).
//!
//! Unsafe code DERIVES a property from a caller-provided value (a
//! one-byte pattern must be ASCII; a searcher was built for THIS string)
//! and skips validating it before an unchecked constructor. The source's
//! origin instance is `str::replace` with a custom `Pattern`
//! (rust-lang/rust #156491): the ASCII fast path assumed a one-byte
//! `Utf8Pattern::StringPattern` must be ASCII — but `StringPattern` held
//! `&[u8]` and `as_utf8_pattern` is a safe method any impl can call — a
//! `0x80` byte pattern corrupted the string into non-UTF-8 and
//! `String::from_utf8_unchecked` laundered it. The fix law, verbatim
//! (FORGE Lab post, CC BY 4.0): "When unsafe code depends on a property
//! of a user-provided implementation, don't take it on faith. Call the
//! implementation once and keep the result, validate what it returns, or
//! encode the property in a type that safe code can't forge... If none of
//! those work, make the trait `unsafe` so implementors explicitly promise
//! to uphold it." (The std fix encoded the property in the type:
//! `StringPattern` now holds `&str`.)
//!
//! ## Why suggest-only (never a fix)
//!
//! The three fixes are a design ladder, not a rewrite: add the validation
//! (a check + an error path the fn may not have), re-type the property
//! (an API change every caller feels), or make the trait `unsafe` (an
//! ecosystem contract change). NO `FIXABLE_SEC` membership, NO
//! `collect_edits` arm.
//!
//! ## The bounded form
//!
//! ```text
//! pub fn swap_byte<P: EncodedPattern>(haystack: &str, from: &P, to: u8) -> String {
//!     let b = from.byte_at();                       // derived property: 1 byte
//!     unsafe { String::from_utf8_unchecked(
//!         haystack.bytes().map(|x| if x == b { to } else { x }).collect(),
//!     ) }                                           // the property never checked
//! }
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Unchecked constructor (gate a)**: the fn body contains
//!   `from_utf8_unchecked` / `from_utf8_unchecked_mut` (the UTF-8
//!   validity property).
//! - **Derived operand (gate b, either arm)**: the fn's generics carry a
//!   provider-trait bound (`AsRef` / `AsMut` / `Borrow` / `BorrowMut` /
//!   `Deref` — the caller chooses the impl), OR the constructor's
//!   argument expressions contain a method call (`collect()` /
//!   `.iter()` / a transform chain — the bytes were DERIVED from
//!   caller-provided behavior, not constructed from literals).
//! - **No validation in scope (gate c)**: the fn contains no
//!   `from_utf8(` / `is_ascii` / `debug_assert` / `assert!` token — the
//!   property is taken on faith.
//! - `#[cfg(test)]` mods are skipped; `Stmt::Macro` bodies are never
//!   descended into (house discipline); nested fns are their own scope.
//!
//! ## Honest recall limits (stated in-rule)
//!
//! - **Per-fn only, property invisible**: WHICH property the unchecked
//!   constructor needs and whether the derivation actually preserves it
//!   is invisible — the rule flags the trust QUESTION. A derivation that
//!   holds the property BY CONSTRUCTION (the post's type-encoding fix)
//!   still fires as the question; the honest suppressions are the
//!   validation call itself (gate c) or a type the caller cannot forge.
//! - The provider-bound arm keys on the bounded GENERIC, not on whether
//!   that specific argument flows into the constructor — a fn whose
//!   bound exists but whose unchecked bytes never touch the impl fires
//!   as the same question; review decides.
//! - Non-UTF-8 unchecked constructors (`CStr::from_ptr`,
//!   `Box::from_raw`, ...) are out of the bounded shape — vocabulary-
//!   gated (the FFI-hygiene family owns several of their classes).
//! - Cross-fn derivations (bytes built in a helper, laundered here)
//!   unconnected.
//! - Distinct-from `unchecked-wire-arithmetic` (Batch 165 — the overflow
//!   sink on arithmetic; THIS owns the validity-property sink) and from
//!   `unsafe-invariant-panic-exposed` (Batch 353 sibling — the unwind
//!   axis; THIS owns the derived-VALUE axis, no panic required).
//! - Batch 353 / Plan 363 / Research 397 (Microsoft FORGE Lab, "Memory
//!   Safety Bugs in Rust Standard Library", 2026-11-09, CC BY 4.0 —
//!   cited as class provenance; the fixture is an original synthesis).

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{File, Item};

/// One detected unchecked constructor over a derived operand.
#[derive(Debug, Clone)]
pub struct DerivedPropertyFinding {
    /// The fn carrying the unchecked constructor.
    pub fn_name: String,
    /// Source line of the unchecked constructor.
    pub line: usize,
    /// The flagged shape (retrieval token).
    pub kind: &'static str,
}

/// The flagged shape (single source, tested).
pub const KIND: &str = "derived-property-unchecked-constructor-no-validation";

/// The unchecked-constructor vocabulary (gate a).
const UNCHECKED_CTORS: &[&str] = &["from_utf8_unchecked"];
/// Provider-trait bounds (gate b, bound arm) — generic-bound tokens.
const PROVIDER_BOUNDS: &[&str] = &["AsRef<", "AsMut<", "Borrow<", "BorrowMut<", "Deref"];
/// Validation vocabulary (gate c) — any of these in the fn suppresses.
const VALIDATION_TOKENS: &[&str] = &["from_utf8(", "is_ascii", "debug_assert", "assert!"];

/// Detect unchecked constructors over derived operands (suggest-only; no
/// edits are produced by this module, ever).
#[must_use]
pub fn detect(_source: &str, file: &File) -> Vec<DerivedPropertyFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(item, &mut findings);
    }
    findings
}

fn collect_in_item(item: &Item, findings: &mut Vec<DerivedPropertyFinding>) {
    match item {
        Item::Fn(f) => scan_fn(&f.sig, &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(&m.sig, &m.block, findings);
                }
            }
        }
        Item::Trait(tr) => {
            for ti in &tr.items {
                if let syn::TraitItem::Fn(m) = ti {
                    if let Some(block) = &m.default {
                        scan_fn(&m.sig, block, findings);
                    }
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline).
            if m.attrs.iter().any(|a| {
                a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test")
            }) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(nested, findings);
                }
            }
        }
        _ => {}
    }
}

fn scan_fn(sig: &syn::Signature, block: &syn::Block, findings: &mut Vec<DerivedPropertyFinding>) {
    // Gate c first (the validation screen). Compact before matching —
    // token streams render with spaces; `from_utf8(` (checked) must NOT
    // match inside `from_utf8_unchecked(` (it cannot: the next char is `_`).
    let body_compact: String = {
        let ts = block.to_token_stream().to_string();
        ts.chars().filter(|c| !c.is_whitespace()).collect()
    };
    if VALIDATION_TOKENS.iter().any(|t| body_compact.contains(t)) {
        return;
    }
    // Gate a: the unchecked constructor must be present.
    if !UNCHECKED_CTORS.iter().any(|t| body_compact.contains(t)) {
        return;
    }

    // Gate b, bound arm: a provider-trait bound on the fn's generics.
    let sig_compact: String = {
        let ts = sig.to_token_stream().to_string();
        ts.chars().filter(|c| !c.is_whitespace()).collect()
    };
    let bound_hit = PROVIDER_BOUNDS.iter().any(|t| sig_compact.contains(t));

    // Gate b, derivation arm + the constructor sites themselves.
    let mut v = CtorVisitor::default();
    v.visit_block(block);
    for site in &v.sites {
        if bound_hit || site.arg_derived {
            findings.push(DerivedPropertyFinding {
                fn_name: sig.ident.to_string(),
                line: site.line,
                kind: KIND,
            });
        }
    }
}

/// One unchecked-constructor call site.
struct CtorSite {
    line: usize,
    /// Whether the argument expressions carry a method call (a derivation).
    arg_derived: bool,
}

/// Finds the unchecked-constructor call sites.
#[derive(Default)]
struct CtorVisitor {
    sites: Vec<CtorSite>,
}

impl Visit<'_> for CtorVisitor {
    fn visit_expr_call(&mut self, c: &syn::ExprCall) {
        let compact: String = {
            let ts = c.to_token_stream().to_string();
            ts.chars().filter(|c| !c.is_whitespace()).collect()
        };
        if UNCHECKED_CTORS.iter().any(|t| compact.contains(t)) {
            struct ArgScan {
                method_seen: bool,
            }
            impl Visit<'_> for ArgScan {
                fn visit_expr_method_call(&mut self, _m: &syn::ExprMethodCall) {
                    self.method_seen = true;
                }
                fn visit_item(&mut self, _i: &Item) {}
            }
            let mut args = ArgScan { method_seen: false };
            for a in &c.args {
                args.visit_expr(a);
            }
            self.sites.push(CtorSite {
                line: c.func.span().start().line,
                arg_derived: args.method_seen,
            });
        }
        syn::visit::visit_expr_call(self, c);
    }

    fn visit_expr_method_call(&mut self, m: &syn::ExprMethodCall) {
        let name = m.method.to_string();
        if UNCHECKED_CTORS.contains(&name.as_str()) {
            self.sites.push(CtorSite {
                line: m.method.span().start().line,
                // A method-call constructor's receiver is by definition a
                // derived operand.
                arg_derived: true,
            });
        }
        syn::visit::visit_expr_method_call(self, m);
    }

    fn visit_item(&mut self, _i: &Item) {
        // Never descend past the fn boundary.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings_of(src: &str) -> Vec<DerivedPropertyFinding> {
        let file = syn::parse_file(src).expect("parses");
        detect(src, &file)
    }

    #[test]
    fn fires_on_derived_chain_into_unchecked_ctor() {
        let src = r#"
            pub fn swap_byte<P: EncodedPattern>(haystack: &str, from: &P, to: u8) -> String {
                let b = from.byte_at();
                unsafe {
                    String::from_utf8_unchecked(
                        haystack.bytes().map(|x| if x == b { to } else { x }).collect(),
                    )
                }
            }
        "#;
        let f = findings_of(src);
        assert_eq!(f.len(), 1, "the derived chain must fire once");
        assert_eq!(f[0].kind, KIND);
        assert_eq!(f[0].fn_name, "swap_byte");
    }

    #[test]
    fn fires_on_provider_bound_with_plain_operand() {
        let src = r#"
            pub fn render<P: Borrow<str>>(piece: &P) -> String {
                let mut buf: Vec<u8> = Vec::new();
                buf.extend_from_slice(piece.borrow().as_bytes());
                unsafe { String::from_utf8_unchecked(buf) }
            }
        "#;
        // The bound arm: the caller chooses `Borrow`'s impl — the bytes
        // are trusted even where the argument itself is a plain local.
        assert_eq!(findings_of(src).len(), 1);
    }

    #[test]
    fn silent_when_validation_is_present() {
        let src = r#"
            pub fn swap_byte<P: EncodedPattern>(haystack: &str, from: &P, to: u8) -> String {
                let b = from.byte_at();
                debug_assert!(b.is_ascii(), "pattern byte must be ascii");
                unsafe {
                    String::from_utf8_unchecked(
                        haystack.bytes().map(|x| if x == b { to } else { x }).collect(),
                    )
                }
            }
        "#;
        // The validation law satisfied in-scope (debug_assert + is_ascii).
        assert!(findings_of(src).is_empty());
    }

    #[test]
    fn silent_when_the_checked_path_is_used() {
        let src = r#"
            pub fn swap_byte_checked<P: EncodedPattern>(
                haystack: &str,
                from: &P,
                to: u8,
            ) -> Option<String> {
                let b = from.byte_at();
                let bytes: Vec<u8> = haystack
                    .bytes()
                    .map(|x| if x == b { to } else { x })
                    .collect();
                String::from_utf8(bytes).ok()
            }
        "#;
        // No unchecked constructor in the fn at all.
        assert!(findings_of(src).is_empty());
    }

    #[test]
    fn silent_on_literal_derivation_without_provider_bound() {
        let src = r#"
            pub fn banner_bytes() -> String {
                let bytes: Vec<u8> = b"READY".iter().copied().collect();
                unsafe { String::from_utf8_unchecked(bytes) }
            }
        "#;
        // No provider bound and the constructor's own argument is a plain
        // local: no caller-provided impl is in the loop at this fn.
        assert!(findings_of(src).is_empty());
    }

    #[test]
    fn test_mods_are_skipped() {
        let src = r#"
            #[cfg(test)]
            mod tests {
                #[test]
                fn t() {
                    let bytes: Vec<u8> = vec![0x41];
                    let s = unsafe { String::from_utf8_unchecked(bytes) };
                    assert_eq!(s, "A");
                }
            }
        "#;
        assert!(findings_of(src).is_empty(), "cfg(test) habitat is skipped");
    }
}

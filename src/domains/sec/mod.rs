//! The `sec` domain — security-pattern healing (Proposal 004, Plan 018).
//!
//! Fourth retrieval `HealDomain` (after clippy/perf/kernel; `rustc_errors`
//! is a driver-based surface, not a retrieval domain). Scope:
//! security-relevant **source-code patterns with bounded fix-spaces** —
//! FFI-boundary safety, Drop/panic safety, raw-pointer hygiene,
//! reference-count leak shapes.
//!
//! ## What this domain is NOT (Proposal 004, non-negotiable)
//!
//! - NOT dependency auditing (cargo-audit/cargo-deny own that).
//! - NOT a lint engine (dylint exists; our value is corpus + heal loop +
//!   modelless retrieval + bounded-fix proposers).
//! - NOT judgment-bearing detection (a wrong security "fix" is worse than
//!   no fix — every rule is parse-only provable with a bounded fix, or is
//!   suggest-only).
//!
//! ## Phase 1 territory (measured on clippy 1.93.0 — see the batch research
//! note for the full probe evidence table)
//!
//! Six rules survived mine-time falsification (5 of 11 candidates killed).
//! Boundary vs clippy/rustc lints is measured, not assumed:
//! `improper_ctypes*` owns the extern-block DECLARATION side (killed two
//! candidates); `uninit_assumed_init` owns the direct-chain uninit form
//! (error-level on 1.93); `unwrap_used` fires on Drop fixtures but offers
//! NO fix — our Drop rule is drop-glue-scoped WITH a bounded fix, not an
//! equivalent-fix duplication.

use serde::{Deserialize, Serialize};

use crate::domain::{HealDomain, HealEntry};

pub mod corpus;
pub mod proposers;

/// The rule family (parallel to clippy's `lint_group`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecFamily {
    /// FFI-boundary safety (panic containment, ABI hygiene).
    Ffi,
    /// Drop/panic/leak safety (drop glue that can abort, leaked refs).
    DropLeak,
    /// Raw-pointer + unsafe-block hygiene.
    UnsafeHygiene,
    /// Crypto-API hygiene (Phase 2 — none shipped yet).
    CryptoHygiene,
    /// Sort/ordering panic safety (P8 — a comparator violating the
    /// total-order contract makes the std sorts PANIC since 1.81 and
    /// silently mis-orders NaN otherwise).
    PanicSafety,
    /// Input-validation safety (Batch 131 — untrusted lengths/sizes flowing
    /// into allocations, secret files created without permission hardening).
    /// Added APPEND-ONLY: no existing variant renumbered.
    InputValidation,
    /// Float-numerics correctness (Issue 185 — the f32 Softmax Collapse
    /// family, arXiv:2610.31642: `p_t - 1` / `sigmoid(z) - 1` / `-ln p_t`
    /// round to exactly 0.0 past the ~16.6 margin cliff). Valid code, silent
    /// clippy — the `PanicSafety` precedent's shape (a float defect, one
    /// mechanical rewrite, suggest-only siblings). Added APPEND-ONLY.
    Numerics,
}

impl SecFamily {
    /// Stable label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Ffi => "ffi",
            Self::DropLeak => "drop-leak",
            Self::UnsafeHygiene => "unsafe-hygiene",
            Self::CryptoHygiene => "crypto-hygiene",
            Self::PanicSafety => "panic-safety",
            Self::InputValidation => "input-validation",
            Self::Numerics => "numerics",
        }
    }
}

/// One sec rule: buggy/fixed fixture pair + family (mirrors `ClippyFixEntry`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecRule {
    /// The rule id (kebab-case, the `rust_perf` convention).
    pub rule_id: String,
    /// The security concept in one line (parallel to `gpu_concept` — the
    /// `kernel_opt` vocabulary).
    pub sec_concept: String,
    /// The buggy code that triggers the rule.
    pub buggy_code: String,
    /// The fixed code that resolves the rule.
    pub fixed_code: String,
    /// The family.
    pub family: SecFamily,
}

impl HealEntry for SecRule {
    #[inline]
    fn rule_id(&self) -> &str {
        &self.rule_id
    }

    #[inline]
    fn buggy_code(&self) -> &str {
        &self.buggy_code
    }

    #[inline]
    fn fixed_code(&self) -> &str {
        &self.fixed_code
    }
}

/// The parse-only verdict (always available).
///
/// `unsafe_count_delta` comes from the domain's own geiger-provenance
/// counter ([`count_unsafe`]) — fixed-minus-buggy. Most sec fixes are
/// unsafe-count-neutral (guards + wraps); the ref→pointer rewrite adds one
/// unsafe block by design. It is a SIGNAL for review, never a gate.
#[derive(Debug, Clone)]
pub struct SecVerdict {
    /// Whether the proposed fix parses as valid Rust.
    pub parses: bool,
    /// `count_unsafe(fixed) - count_unsafe(buggy)` when both parse.
    pub unsafe_count_delta: Option<i64>,
}

/// The sec healing domain — `HealDomain` impl.
#[derive(Debug, Clone, Copy, Default)]
pub struct SecDomain;

impl HealDomain for SecDomain {
    type Entry = SecRule;
    type Verdict = SecVerdict;

    fn build_corpus() -> Vec<Self::Entry> {
        corpus::build_sec_seed_corpus().entries
    }

    fn validate(rule_id: &str, proposed_fix: &str) -> Self::Verdict {
        let Ok(fixed_file) = syn::parse_file(proposed_fix) else {
            return SecVerdict {
                parses: false,
                unsafe_count_delta: None,
            };
        };
        let delta = corpus::build_sec_seed_corpus()
            .entries
            .into_iter()
            .find(|r| r.rule_id == rule_id)
            .and_then(|r| syn::parse_file(&r.buggy_code).ok())
            .map(|buggy_file| {
                let before = count_unsafe(&buggy_file).total();
                let after = count_unsafe(&fixed_file).total();
                i64::from(after) - i64::from(before)
            });
        SecVerdict {
            parses: true,
            unsafe_count_delta: delta,
        }
    }
}

/// Unsafe-usage counts per the cargo-geiger taxonomy (T0.5) — our own syn
/// visitor, zero new dependencies (we already parse every file).
///
/// Counts: unsafe fns + methods, attribute-unsafety (a safe fn carrying
/// `#[no_mangle]` / `#[export_name]` is externally callable regardless of
/// its `unsafe` marker — geiger's counting nugget), unsafe impls, unsafe
/// traits, and unsafe expressions (blocks at any nesting depth).
/// `#![forbid(unsafe_code)]` detection lives in [`file_forbids_unsafe`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnsafeCounts {
    /// `unsafe fn` items (free + methods, any nesting depth).
    pub unsafe_fns: u32,
    /// Safe fns carrying `#[no_mangle]` / `#[export_name]`.
    pub attribute_unsafe_fns: u32,
    /// `unsafe impl` items.
    pub unsafe_impls: u32,
    /// `unsafe trait` items.
    pub unsafe_traits: u32,
    /// Unsafe expressions/blocks (incl. inside fn bodies + methods).
    pub unsafe_exprs: u32,
}

impl UnsafeCounts {
    /// The headline total (what `unsafe_count_delta` diffs).
    #[must_use]
    pub fn total(&self) -> u32 {
        self.unsafe_fns
            + self.attribute_unsafe_fns
            + self.unsafe_impls
            + self.unsafe_traits
            + self.unsafe_exprs
    }
}

/// Whether the file carries `#![forbid(unsafe_code)]` (the clean-slate mark).
#[must_use]
pub fn file_forbids_unsafe(file: &syn::File) -> bool {
    file.attrs.iter().any(|a| {
        a.path().is_ident("forbid")
            && match &a.meta {
                syn::Meta::List(ml) => ml.tokens.to_string().contains("unsafe_code"),
                _ => false,
            }
    })
}

/// Count unsafe usage per the geiger taxonomy (see [`UnsafeCounts`]).
#[must_use]
pub fn count_unsafe(file: &syn::File) -> UnsafeCounts {
    use syn::visit::Visit;
    use syn::ItemFn;

    struct Counter(UnsafeCounts);

    fn is_attr_unsafe(f: &ItemFn) -> bool {
        // Attribute-unsafety counts SAFE fns only (an unsafe fn is already
        // counted by its marker).
        f.sig.unsafety.is_none()
            && f.attrs
                .iter()
                .any(|a| a.path().is_ident("no_mangle") || a.path().is_ident("export_name"))
    }

    impl<'ast> Visit<'ast> for Counter {
        fn visit_item_fn(&mut self, f: &'ast ItemFn) {
            if f.sig.unsafety.is_some() {
                self.0.unsafe_fns += 1;
            } else if is_attr_unsafe(f) {
                self.0.attribute_unsafe_fns += 1;
            }
            syn::visit::visit_item_fn(self, f);
        }

        fn visit_impl_item_fn(&mut self, m: &'ast syn::ImplItemFn) {
            if m.sig.unsafety.is_some() {
                self.0.unsafe_fns += 1;
            }
            syn::visit::visit_impl_item_fn(self, m);
        }

        fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
            if i.unsafety.is_some() {
                self.0.unsafe_impls += 1;
            }
            syn::visit::visit_item_impl(self, i);
        }

        fn visit_item_trait(&mut self, t: &'ast syn::ItemTrait) {
            if t.unsafety.is_some() {
                self.0.unsafe_traits += 1;
            }
            syn::visit::visit_item_trait(self, t);
        }

        fn visit_expr_unsafe(&mut self, u: &'ast syn::ExprUnsafe) {
            self.0.unsafe_exprs += 1;
            syn::visit::visit_expr_unsafe(self, u);
        }
    }

    let mut c = Counter(UnsafeCounts::default());
    c.visit_file(file);
    c.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_the_geiger_taxonomy() {
        let f = syn::parse_file(
            "unsafe fn a() {}\n\
             #[no_mangle]\npub extern \"C\" fn b() { unsafe { } }\n\
             unsafe impl Drop for S { fn drop(&mut self) {} }\n\
             unsafe trait T {}",
        )
        .expect("parse");
        let c = count_unsafe(&f);
        assert_eq!(c.unsafe_fns, 1); // `unsafe fn a`
        assert_eq!(c.attribute_unsafe_fns, 1); // #[no_mangle] safe extern fn
        assert_eq!(c.unsafe_impls, 1);
        assert_eq!(c.unsafe_traits, 1);
        assert_eq!(c.unsafe_exprs, 1); // the unsafe block in b
        assert_eq!(c.total(), 5);
    }

    #[test]
    fn forbid_detection() {
        let f = syn::parse_file("#![forbid(unsafe_code)] fn a() {}").expect("parse");
        assert!(file_forbids_unsafe(&f));
        let g = syn::parse_file("#![forbid(clippy::all)] fn a() {}").expect("parse");
        assert!(!file_forbids_unsafe(&g));
    }

    #[test]
    fn validate_parses_and_diffs_unsafe() {
        let v = SecDomain::validate(
            "ffi-extern-fn-catch-unwind",
            "#[no_mangle]\npub extern \"C\" fn on_packet(n: u32) {\n    let _ = std::panic::catch_unwind(|| {\n        let v = vec![n; 4];\n        println!(\"{v:?}\");\n    });\n}\n",
        );
        assert!(v.parses);
        // The catch_unwind wrap is unsafe-count-neutral.
        assert_eq!(v.unsafe_count_delta, Some(0));
    }

    #[test]
    fn validate_rejects_non_parsing() {
        let v = SecDomain::validate("ffi-extern-fn-catch-unwind", "fn broken( {{{");
        assert!(!v.parses);
        assert_eq!(v.unsafe_count_delta, None);
    }
}

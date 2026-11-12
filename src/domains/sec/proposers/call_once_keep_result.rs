//! `user-impl-call-once-keep-result` — SUGGEST-ONLY detector
//! (Batch 353, the FORGE Lab std memory-safety post distill; Research 397 /
//! Plan 363).
//!
//! Invariant-carrying code calls a caller-provided safe trait method TWICE
//! and assumes the results agree (same length, same bytes, same slice).
//! A safe impl may return a different value on each call — the bounds
//! check passes against the first answer while the access uses the
//! second, and the disagreement becomes an out-of-bounds read. Source
//! shape, verbatim (FORGE Lab post, CC BY 4.0): a helper that returns the
//! byte at `idx` from "any type that can be viewed as a byte slice"
//! "looks correct, but it assumes that two calls to `as_ref` return the
//! same slice. Because `S` is chosen by the caller, a safe `AsRef`
//! implementation can return a long slice the first time and an empty one
//! the second, and `get` reads out of bounds." Origin instances: the post's
//! `get`/`AsRef` opening example, `join` trusting `Borrow::borrow`
//! twice (CVE-2020-36323, the 2020 shrink-path fix, plus the 2026 grow-path
//! re-fix rust-lang/rust#155708 — "if it grows on the second call, the
//! remaining-length calculation underflows into a heap overflow"). The fix
//! law, verbatim: "Call the implementation once and keep the result."
//!
//! ## Why suggest-only (never a fix)
//!
//! The call-once rewrite rebinds a local and reshapes borrow scopes — a
//! mechanical but review-bearing change (which of the two calls is the
//! authoritative one is a semantic decision). NO `FIXABLE_SEC` membership,
//! NO `collect_edits` arm.
//!
//! ## The bounded form
//!
//! ```text
//! pub fn get<S: AsRef<[u8]>>(s: &S, idx: usize) -> Option<u8> {
//!     if idx < s.as_ref().len() {                 // first call — the check
//!         Some(unsafe { *s.as_ref().get_unchecked(idx) }) // second call — the access
//!     } else { None }
//! }
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Provider-trait bound (gate a)**: the fn's generics carry
//!   `AsRef<` / `AsMut<` / `Borrow<` / `BorrowMut<` — the caller chooses
//!   the impl, consistency is NOT part of `AsRef`'s contract (`Borrow`
//!   documents Eq-consistency, but the join bugs show even documented
//!   contracts rot; both fire).
//! - **Repeated call on the SAME receiver (gate b)**: the same
//!   method name (`.as_ref` / `.as_mut` / `.borrow` / `.borrow_mut`) on
//!   the same receiver expression occurs ≥ 2 times in the body —
//!   receiver identity is the compact token text of the receiver
//!   expression; two different receivers are two independent calls and
//!   stay silent.
//! - **Invariant-carrying context (gate c)**: the body contains an
//!   `unsafe` token or a `.len()` call (bounds/arithmetic vocabulary —
//!   the disagreement reaches a length calculation or an unchecked
//!   access).
//! - `#[cfg(test)]` mods are skipped; `Stmt::Macro` bodies are never
//!   descended into (house discipline); nested fns are their own scope.
//!
//! ## Honest recall limits (stated in-rule)
//!
//! - **Per-fn only**: whether the impl actually disagrees is invisible —
//!   the rule flags the trust QUESTION, not a proven OOB.
//! - The trusted trait bound must be ON THE FN (a helper receiving
//!   `&[u8]` after the caller's `as_ref()` is the FIXED shape and stays
//!   silent); cross-fn disagreements unconnected.
//! - Distinct-from `read-output-trust-inherits-argument-taint` (Batch
//!   340): THAT owns the TAINT axis (a read's output inherits its
//!   arguments' trust); THIS owns the CONSISTENCY axis (two calls to one
//!   impl must agree). Distinct-from `unsafe-impl-send-sync-unsound-field`
//!   (the MARKER axis: `unsafe impl Sync` promising thread-safety) — both
//!   are trusted-impl rules; the adjudication is in-rule.
//! - Batch 353 / Plan 363 / Research 397 (Microsoft FORGE Lab, "Memory
//!   Safety Bugs in Rust Standard Library", 2026-11-09, CC BY 4.0 —
//!   cited as class provenance; the fixture is an original synthesis).

use quote::ToTokens;
use syn::visit::Visit;
use syn::{File, Item};

/// One detected repeat-call site on a provider-trait receiver.
#[derive(Debug, Clone)]
pub struct CallOnceFinding {
    /// The fn carrying the repeated call.
    pub fn_name: String,
    /// Source line of the SECOND call.
    pub line: usize,
    /// The flagged shape (retrieval token).
    pub kind: &'static str,
}

/// The flagged shape (single source, tested).
pub const KIND: &str = "provider-impl-called-twice-results-assumed-agreeing";

/// Provider-trait bounds (gate a) — generic-bound tokens matched on the
/// compact signature text.
const PROVIDER_BOUNDS: &[&str] = &["AsRef<", "AsMut<", "Borrow<", "BorrowMut<"];
/// The trusted methods (gate b) — method names.
const TRUSTED_METHODS: &[&str] = &["as_ref", "as_mut", "borrow", "borrow_mut"];
/// Invariant-carrying vocabulary (gate c).
const CONTEXT_TOKENS: &[&str] = &["unsafe", ".len("];

/// Detect repeated provider-impl calls on one receiver (suggest-only; no
/// edits are produced by this module, ever).
#[must_use]
pub fn detect(_source: &str, file: &File) -> Vec<CallOnceFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(item, &mut findings);
    }
    findings
}

fn collect_in_item(item: &Item, findings: &mut Vec<CallOnceFinding>) {
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

fn scan_fn(sig: &syn::Signature, block: &syn::Block, findings: &mut Vec<CallOnceFinding>) {
    // Gate a: a provider-trait bound on the fn's own generics. Compact
    // before matching — token streams render with spaces.
    let sig_compact: String = {
        let ts = sig.to_token_stream().to_string();
        ts.chars().filter(|c| !c.is_whitespace()).collect()
    };
    if !PROVIDER_BOUNDS.iter().any(|t| sig_compact.contains(t)) {
        return;
    }

    // Gate c: invariant-carrying context.
    let body_compact: String = {
        let ts = block.to_token_stream().to_string();
        ts.chars().filter(|c| !c.is_whitespace()).collect()
    };
    if !CONTEXT_TOKENS.iter().any(|t| body_compact.contains(t)) {
        return;
    }

    // Gate b: per-(receiver, method) call counts with the second call's line.
    let mut v = CallVisitor {
        counts: Vec::new(),
        repeats: Vec::new(),
    };
    v.visit_block(block);
    for r in &v.repeats {
        findings.push(CallOnceFinding {
            fn_name: sig.ident.to_string(),
            line: r.line,
            kind: KIND,
        });
    }
}

struct Repeat {
    line: usize,
}

/// Counts `.as_ref()`-family calls per (receiver text, method name).
struct CallVisitor {
    counts: Vec<((String, String), usize)>,
    repeats: Vec<Repeat>,
}

impl Visit<'_> for CallVisitor {
    fn visit_expr_method_call(&mut self, m: &syn::ExprMethodCall) {
        let name = m.method.to_string();
        if TRUSTED_METHODS.contains(&name.as_str()) {
            let receiver = m.receiver.to_token_stream().to_string();
            let key = (receiver, name);
            let line = m.method.span().start().line;
            match self.counts.iter_mut().find(|(k, _)| *k == key) {
                Some((_, c)) => {
                    *c += 1;
                    self.repeats.push(Repeat { line });
                }
                None => self.counts.push((key, 1)),
            }
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

    fn findings_of(src: &str) -> Vec<CallOnceFinding> {
        let file = syn::parse_file(src).expect("parses");
        detect(src, &file)
    }

    #[test]
    fn fires_on_as_ref_called_twice_with_bounds_check() {
        let src = r#"
            pub fn get<S: AsRef<[u8]>>(s: &S, idx: usize) -> Option<u8> {
                if idx < s.as_ref().len() {
                    Some(unsafe { *s.as_ref().get_unchecked(idx) })
                } else {
                    None
                }
            }
        "#;
        let f = findings_of(src);
        assert_eq!(f.len(), 1, "the repeated call must fire once");
        assert_eq!(f[0].kind, KIND);
        assert_eq!(f[0].fn_name, "get");
    }

    #[test]
    fn fires_on_borrow_called_twice_with_len_arithmetic() {
        let src = r#"
            pub fn remaining_len<K: Borrow<str>>(items: &[K]) -> usize {
                let mut total = 0;
                for k in items {
                    total += k.borrow().len();
                    total += k.borrow().chars().count();
                }
                total
            }
        "#;
        assert_eq!(findings_of(src).len(), 1);
    }

    #[test]
    fn silent_when_called_once_and_kept() {
        let src = r#"
            pub fn get<S: AsRef<[u8]>>(s: &S, idx: usize) -> Option<u8> {
                let bytes = s.as_ref();
                if idx < bytes.len() {
                    Some(unsafe { *bytes.get_unchecked(idx) })
                } else {
                    None
                }
            }
        "#;
        // The call-once fix: one call, the result kept — single textual
        // occurrence, nothing to disagree.
        assert!(findings_of(src).is_empty());
    }

    #[test]
    fn silent_on_two_different_receivers() {
        let src = r#"
            pub fn join_lengths<A: AsRef<str>, B: AsRef<str>>(a: &A, b: &B) -> usize {
                a.as_ref().len() + b.as_ref().len()
            }
        "#;
        // Two receivers, two impls: each called once — per-receiver
        // counting keeps this silent.
        assert!(findings_of(src).is_empty());
    }

    #[test]
    fn silent_without_the_provider_bound() {
        let src = r#"
            pub fn get(s: &[u8], idx: usize) -> Option<u8> {
                if idx < s.len() {
                    Some(unsafe { *s.get_unchecked(idx) })
                } else {
                    None
                }
            }
        "#;
        // The slice arrives already borrowed: the caller's impl ran once,
        // upstream of this fn.
        assert!(findings_of(src).is_empty());
    }

    #[test]
    fn silent_without_invariant_context() {
        let src = r#"
            pub fn show<S: AsRef<str>>(s: &S) -> String {
                let a = s.as_ref().to_string();
                let b = s.as_ref().to_uppercase();
                a + &b
            }
        "#;
        // Two calls, but no unsafe / len vocabulary: the disagreement has
        // no reachable invariant to break here.
        assert!(findings_of(src).is_empty());
    }

    #[test]
    fn test_mods_are_skipped() {
        let src = r#"
            #[cfg(test)]
            mod tests {
                #[test]
                fn t() {
                    fn f<S: AsRef<[u8]>>(s: &S) -> usize {
                        s.as_ref().len() + s.as_ref().len()
                    }
                    assert_eq!(f(&[1u8, 2]), 4);
                }
            }
        "#;
        assert!(findings_of(src).is_empty(), "cfg(test) habitat is skipped");
    }
}

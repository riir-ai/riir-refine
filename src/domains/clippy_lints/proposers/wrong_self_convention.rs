//! `clippy::wrong_self_convention` — `fn into_*(this: &Self)` → receiver by
//! value (Batch 156 / Plan 147 — the frontier-lead mining pack II; family
//! fixture verified to fire exactly this lint under all+pedantic+nursery,
//! 2026-09-18).
//!
//! Pattern: an INHERENT-impl method whose name starts `into_` and whose
//! receiver is a plain `&self`/`&mut self`. The convention (`into_*` consumes
//! self) is expressed by splicing the receiver span to `self`.
//!
//! **VERIFY tier** (type-blind — the `ptr_arg` precedent): parse-only cannot
//! see (a) a body that borrows self past the move (`fn into_label(&self) ->
//! &str { &self.label }` → E0515), (b) callers that pass borrows (E0308 /
//! E0382), or (c) a non-Copy self whose fields are moved conditionally —
//! every failure class is check-visible, reverted by the `--fix --verify`
//! compile gate.
//!
//! v1 scope (conservative, probe-shaped like the fired fixture):
//! - INHERENT impls only (a trait-impl receiver edit is an E0050 signature
//!   mismatch against the trait — always reverted; trait `into_*` impls are
//!   rare);
//! - methods with NO visibility modifier only — real clippy's
//!   `avoid_breaking_exported_api` default skips exported items, and a
//!   receiver flip on a `pub` method is an API break the compile gate is
//!   BLIND to (it compiles) — refused up front (documented divergence:
//!   clippy lints `pub(crate)`, we refuse it too);
//! - no method attributes, no impl attributes (cfg views — the
//!   `needless_return` divergence class);
//! - plain receivers only: `&'a self` (annotated) refuses; receiver attrs
//!   refuse; `self` already-by-value is not a match.
//!
//! The `to_*`/`is_*`/`from_*` convention rows are DELIBERATELY out of scope:
//! `to_*(self)` fires only on non-Copy types (typeck-driven — clippy is
//! type-aware, a parse-only proposer would fire on Copy types where clippy
//! is silent and the rewrite compiles: a silent false positive), and
//! `from_*` taking self is an associated-fn shape change, not a receiver
//! splice.

use syn::visit::Visit;
use syn::{File, ImplItemFn, ItemImpl, Receiver, Visibility};

use super::SourceEdit;
use super::helpers::byte_range;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    // NOTE: no source reads needed — the edit is a fixed replacement over
    // the receiver span; `source` stays in the signature for substrate
    // uniformity (every proposer takes it).
    let _ = source;
    let mut v = Collector { edits: Vec::new() };
    v.visit_file(file);
    v.edits
}

struct Collector {
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        if i.trait_.is_none() && i.attrs.is_empty() {
            for item in &i.items {
                if let syn::ImplItem::Fn(f) = item {
                    if let Some(edit) = receiver_edit(f) {
                        self.edits.push(edit);
                    }
                }
            }
        }
        // impls do not nest in blocks; nested mods are walked by the visitor
        syn::visit::visit_item_impl(self, i);
    }
}

/// The receiver-splice edit for one `into_*(&self)`-shaped method, `None`
/// when any guard refuses.
fn receiver_edit(f: &ImplItemFn) -> Option<SourceEdit> {
    if !f.attrs.is_empty() {
        return None;
    }
    if !matches!(f.vis, Visibility::Inherited) {
        return None; // exported-API divergence (clippy's own default skip)
    }
    if !f.sig.ident.to_string().starts_with("into_") {
        return None;
    }
    let (recv, _) = first_input_receiver(&f.sig)?;
    if !recv.attrs.is_empty() {
        return None;
    }
    // Plain `&self` / `&mut self` — the reference is what the convention
    // flags. The annotated forms refuse (keep the surface minimal, the
    // ptr_arg lifetime guard precedent): `&'a self` rides the lifetime in
    // `reference`; `self: &'a Self` (typed receiver) carries a colon.
    let Some((_, lifetime)) = &recv.reference else {
        return None;
    };
    if lifetime.is_some() || recv.colon_token.is_some() {
        return None;
    }
    let range = byte_range(recv);
    Some(SourceEdit {
        start: range.start,
        end: range.end,
        replacement: "self".to_string(),
    })
}

/// The first input when it is the receiver (`self` must lead a method sig).
fn first_input_receiver(sig: &syn::Signature) -> Option<(&Receiver, &syn::FnArg)> {
    let first = sig.inputs.first()?;
    match first {
        syn::FnArg::Receiver(r) => Some((r, first)),
        syn::FnArg::Typed(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const FIXTURE: &str = "struct Sig { code: u32 }\nimpl Sig {\n    fn into_label(&self) -> String { format!(\"sig-{}\", self.code) }\n}";

    #[test]
    fn rewrites_ref_self_receiver() {
        let out = propose_fix("wrong_self_convention", FIXTURE).expect("matched");
        assert!(
            out.contains("fn into_label(self) -> String"),
            "receiver not spliced: {out}"
        );
        assert!(
            !out.contains("into_label(&self)"),
            "old receiver must be gone: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn rewrites_ref_mut_receiver() {
        let input = "struct Sig { code: u32 }\nimpl Sig {\n    fn into_slug(&mut self) -> String { format!(\"sig-{}\", self.code) }\n}";
        let out = propose_fix("wrong_self_convention", input).expect("matched");
        assert!(
            out.contains("fn into_slug(self) -> String"),
            "receiver not spliced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn body_rides_verbatim() {
        let out = propose_fix("wrong_self_convention", FIXTURE).expect("matched");
        assert!(
            out.contains("format!(\"sig-{}\", self.code)"),
            "body must stay byte-identical: {out}"
        );
        assert!(
            out.contains("struct Sig { code: u32 }"),
            "impl head must stay: {out}"
        );
    }

    #[test]
    fn by_value_receiver_untouched() {
        let input = "struct Sig { code: u32 }\nimpl Sig {\n    fn into_label(self) -> String { format!(\"sig-{}\", self.code) }\n}";
        let out = propose_fix("wrong_self_convention", input);
        assert!(
            out.is_none(),
            "already-conventional shape must not fire: {out:?}"
        );
    }

    #[test]
    fn non_into_prefix_untouched() {
        let input = "struct Sig { code: u32 }\nimpl Sig {\n    fn label(&self) -> String { format!(\"sig-{}\", self.code) }\n}";
        let out = propose_fix("wrong_self_convention", input);
        assert!(out.is_none(), "non-into_ prefix must not fire: {out:?}");
    }

    #[test]
    fn pub_method_refused() {
        let input = "struct Sig { code: u32 }\nimpl Sig {\n    pub fn into_label(&self) -> String { format!(\"sig-{}\", self.code) }\n}";
        let out = propose_fix("wrong_self_convention", input);
        assert!(
            out.is_none(),
            "exported method must refuse (API break the gate cannot see): {out:?}"
        );
    }

    #[test]
    fn trait_impl_refused() {
        let input = "struct Sig { code: u32 }\ntrait Make { fn into_label(&self) -> String; }\nimpl Make for Sig {\n    fn into_label(&self) -> String { format!(\"sig-{}\", self.code) }\n}";
        let out = propose_fix("wrong_self_convention", input);
        assert!(
            out.is_none(),
            "trait-impl receiver edit is an E0050 mismatch — refused: {out:?}"
        );
    }

    #[test]
    fn attributed_method_refused() {
        let input = "struct Sig { code: u32 }\nimpl Sig {\n    #[cfg(unix)]\n    fn into_label(&self) -> String { format!(\"sig-{}\", self.code) }\n}";
        let out = propose_fix("wrong_self_convention", input);
        assert!(
            out.is_none(),
            "cfg'd method must refuse (view divergence): {out:?}"
        );
    }

    #[test]
    fn lifetime_annotated_receiver_refused() {
        let input = "struct Sig { code: u32 }\nimpl Sig {\n    fn into_label<'a>(&'a self) -> String { format!(\"sig-{}\", self.code) }\n}";
        let out = propose_fix("wrong_self_convention", input);
        assert!(out.is_none(), "annotated receiver must refuse: {out:?}");
    }

    #[test]
    fn attributed_impl_refused() {
        let input = "struct Sig { code: u32 }\n#[cfg(test)]\nimpl Sig {\n    fn into_label(&self) -> String { format!(\"sig-{}\", self.code) }\n}";
        let out = propose_fix("wrong_self_convention", input);
        assert!(out.is_none(), "cfg'd impl must refuse: {out:?}");
    }

    #[test]
    fn multiple_into_methods_all_spliced() {
        let input = "struct Sig { code: u32 }\nimpl Sig {\n    fn into_label(&self) -> String { format!(\"sig-{}\", self.code) }\n    fn into_slug(&mut self) -> String { format!(\"s-{}\", self.code) }\n}";
        let out = propose_fix("wrong_self_convention", input).expect("matched");
        assert!(
            out.contains("fn into_label(self)"),
            "method 1 not spliced: {out}"
        );
        assert!(
            out.contains("fn into_slug(self)"),
            "method 2 not spliced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn async_fn_shape_fires() {
        let input = "struct Sig { code: u32 }\nimpl Sig {\n    async fn into_label(&self) -> String { format!(\"sig-{}\", self.code) }\n}";
        let out = propose_fix("wrong_self_convention", input).expect("matched");
        assert!(
            out.contains("async fn into_label(self)"),
            "async shape must splice: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }
}

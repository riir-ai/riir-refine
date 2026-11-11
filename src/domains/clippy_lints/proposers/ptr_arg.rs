//! `clippy::ptr_arg` — `&Vec<T>`/`&String` fn params → `&[T]`/`&str`
//! (Batch 155 / Plan 146 — the frontier-lead mining pack; family fixture
//! verified to fire exactly this lint under all+pedantic+nursery,
//! 2026-09-18).
//!
//! Pattern: an immutable reference param whose pointee is `Vec<T>` or
//! `String`. The param's type span is rewritten to the deref-equivalent
//! borrowed form (`&Vec<i32>` → `&[i32]`, `&String` → `&str`) — the body is
//! NOT rewritten: every receiver call on the old form that resolves through
//! auto-deref (`v.len()`, `v.iter()`, `v.contains(..)`) resolves identically
//! on the new one. The inner type is spliced from the ORIGINAL source text
//! of the pointee (`Vec<Vec<u8>>` → `&[Vec<u8>]`, nested generics verbatim).
//!
//! **VERIFY tier** (type-blind — the `redundant_closure` demotion class):
//! parse-only cannot see (a) Vec-only methods on the body's receiver
//! (`v.capacity()`), or (b) `v` passed to a callee that genuinely expects
//! `&Vec<T>`/`&String` — both are E0308/E0599, check-visible, reverted by the
//! `--fix --verify` compile gate. Real clippy is `MachineApplicable` here
//! (it has the types); the compile gate is what stands in for them.
//!
//! Guards: per-param attributes refused (cfg views — the `needless_return`
//! divergence class); lifetime-annotated refs refused (`&'a Vec<T>` — keep
//! the surface minimal, the plain form is the fired histogram's shape);
//! `&mut` forms refused (the family fixture is the shared-ref shape and the
//! mut twin interacts with reborrow analysis).
//!
//! ⛔ Trait-impl refusal (2026-11-11, the neuron-db sweep's E0053 specimen):
//! a method inside `impl Trait for Type` NEVER fires — the trait's declared
//! signatures are coherence-bound and live anywhere (here: `LatentSpace` in
//! katgpt-core, consumed in riir-neuron-db); rewriting only the impl side
//! (`decode(&self, point: &Vec<f32>)` → `&[f32]`) desyncs it from the
//! trait declaration = E0053, and the file-local parse cannot read the
//! trait's own spellings to prove the rewrite sound. Real clippy also skips
//! trait impls for exactly this reason. Inherent `impl Type` methods keep
//! firing (the impl IS the definition — nothing to desync).

use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{File, Type};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_impl(&mut self, imp: &'ast syn::ItemImpl) {
        // Inherent impls only: `impl Trait for Type` methods are bound by
        // the trait's declared signatures (coherence) — see the module's
        // trait-impl refusal note.
        if imp.trait_.is_none() {
            syn::visit::visit_item_impl(self, imp);
        }
    }

    fn visit_item_trait(&mut self, tr: &'ast syn::ItemTrait) {
        // Trait DECLARATIONS never fire either (same-file or not): an
        // external `impl Trait for Type` may pin the current spellings —
        // the parse cannot see external impls to prove the rewrite sound.
        let _ = tr;
    }

    fn visit_fn_arg(&mut self, arg: &'ast syn::FnArg) {
        let syn::FnArg::Typed(arg) = arg else {
            return; // receiver
        };
        if !arg.attrs.is_empty() {
            return;
        }
        if let Some(replacement) = ptr_replacement(self.source, &arg.ty) {
            self.edits.push(edit(arg.ty.as_ref(), replacement));
        }
    }
}

/// `&Vec<T>` → `&[T]`, `&String` → `&str` — None for everything else. The
/// inner type text is sliced from the pointee's OWN source span, so nested
/// generics ride verbatim.
fn ptr_replacement(source: &str, ty: &Type) -> Option<String> {
    let Type::Reference(r) = ty else {
        return None;
    };
    if r.mutability.is_some() || r.lifetime.is_some() {
        return None;
    }
    let Type::Path(p) = &*r.elem else {
        return None;
    };
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    let seg = &p.path.segments[0];
    let elem_text = src(source, &r.elem)?;
    match seg.ident.to_string().as_str() {
        "String" => Some("&str".to_string()),
        "Vec" => {
            let inner = vec_inner(&elem_text)?;
            Some(format!("&[{inner}]"))
        }
        _ => None,
    }
}

/// `Vec<u8>` (the pointee's source text) → `u8`. None when the text is not
/// the `Vec < .. >` shape this node claimed to be (whitespace tolerated;
/// anything exotic refuses — a wrong inner type is a wrong signature).
fn vec_inner(elem_text: &str) -> Option<&str> {
    let rest = elem_text.strip_prefix("Vec")?;
    let rest = rest.trim_start();
    let inner_and_close = rest.strip_prefix('<')?;
    let close = inner_and_close.rfind('>')?;
    // Exactly one outer `Vec<...>` — the LAST `>` closes it.
    Some(inner_and_close[..close].trim())
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn narrows_vec_param_to_slice() {
        let out = propose_fix(
            "ptr_arg",
            "fn total(v: &Vec<i32>) -> i32 { v.iter().sum() }",
        )
        .expect("matched");
        assert!(
            out.contains("fn total(v: &[i32]) -> i32"),
            "param not narrowed: {out}"
        );
        assert!(out.contains("v.iter().sum()"), "body must stay: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn narrows_string_param_to_str() {
        let out = propose_fix("ptr_arg", "fn greet(name: &String) -> usize { name.len() }")
            .expect("matched");
        assert!(
            out.contains("name: &str"),
            "String not narrowed to str: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn generic_vec_param_spliced_verbatim() {
        let out = propose_fix(
            "ptr_arg",
            "fn first_len(rows: &Vec<String>) -> usize { rows.first().map_or(0, String::len) }",
        )
        .expect("matched");
        assert!(
            out.contains("rows: &[String]"),
            "generic arg not spliced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_generic_vec_spliced_verbatim() {
        let out = propose_fix("ptr_arg", "fn flat(v: &Vec<Vec<u8>>) -> usize { v.len() }")
            .expect("matched");
        assert!(
            out.contains("v: &[Vec<u8>]"),
            "nested generic must ride verbatim: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn plain_slice_param_untouched() {
        let out = propose_fix("ptr_arg", "fn total(v: &[i32]) -> i32 { v.iter().sum() }");
        assert!(
            out.is_none(),
            "already-narrowed param must not fire: {out:?}"
        );
    }

    #[test]
    fn mut_ref_param_refused() {
        let out = propose_fix("ptr_arg", "fn push_all(v: &mut Vec<u8>) { v.push(1) }");
        assert!(out.is_none(), "&mut form must be refused: {out:?}");
    }

    #[test]
    fn lifetime_annotated_refused() {
        let out = propose_fix(
            "ptr_arg",
            "fn keep<'a>(v: &'a Vec<u8>) -> usize { v.len() }",
        );
        assert!(
            out.is_none(),
            "lifetime-annotated form must be refused: {out:?}"
        );
    }

    #[test]
    fn owned_and_other_types_untouched() {
        let out = propose_fix("ptr_arg", "fn take(v: Vec<u8>) -> usize { v.len() }");
        assert!(out.is_none(), "owned param must not fire: {out:?}");
        let out2 = propose_fix("ptr_arg", "fn take(v: &u8) -> u8 { *v }");
        assert!(out2.is_none(), "&u8 param must not fire: {out2:?}");
    }

    #[test]
    fn fn_level_attrs_ride() {
        // The guard is per-PARAM attrs (cfg'd params); fn-level attrs ride.
        let input = "#[cfg(unix)]\nfn unix_only(v: &Vec<u8>) -> usize { v.len() }";
        let out = propose_fix("ptr_arg", input).expect("matched");
        assert!(
            out.contains("&[u8]"),
            "fn-attr shape must still fire: {out}"
        );
        assert!(out.contains("#[cfg(unix)]"), "attr must survive: {out}");
    }

    #[test]
    fn impl_method_param_narrows() {
        let input = "struct S;\nimpl S {\n    fn any_zero(&self, v: &Vec<u8>) -> bool { v.contains(&0) }\n}";
        let out = propose_fix("ptr_arg", input).expect("matched");
        assert!(
            out.contains("v: &[u8]"),
            "impl method param must narrow: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn trait_impl_method_declined() {
        // The neuron-db sweep specimen (2026-11-11): `impl LatentSpace for
        // KarcWoutSpace` — the trait lives in katgpt-core; rewriting only
        // the impl's `&Vec<f32>` params desyncs the coherence-bound
        // signatures (E0053). MUST NOT fire.
        let input = concat!(
            "struct K;\n",
            "trait LatentSpace {\n",
            "    fn decode(&self, point: &Vec<f32>) -> Vec<f32>;\n",
            "}\n",
            "impl LatentSpace for K {\n",
            "    fn decode(&self, point: &Vec<f32>) -> Vec<f32> { point.clone() }\n",
            "}\n",
        );
        let out = propose_fix("ptr_arg", input);
        assert!(out.is_none(), "trait-impl method must decline: {out:?}");
    }

    #[test]
    fn trait_declaration_declined() {
        // A same-file trait DECLARATION never fires either — an external
        // impl may pin the declared spellings; the parse cannot see it.
        let input = "trait T {\n    fn push_all(&self, v: &Vec<u8>) -> usize;\n}";
        let out = propose_fix("ptr_arg", input);
        assert!(out.is_none(), "trait declaration must decline: {out:?}");
    }
}

//! `ffi-extern-fn-catch-unwind` — wrap unit-return `extern` fn bodies in
//! `catch_unwind` (panic containment at the FFI boundary).
//!
//! A panic unwinding across an `extern "C"` boundary is UB (the caller —
//! C, Unity, WASM — has no landing pad). The in-house `#[riir_ffi]` macro
//! enforces this in exactly one crate (riir-viewbridge, Plan 532 §9); this
//! rule generalizes it to any unit-return extern-ABI fn whose body does not
//! already contain `catch_unwind`.
//!
//! ## Gates (conservative — a wrong sec fix is worse than no fix)
//! - **extern ABI + unit return only**: `sig.abi` present AND the return
//!   type is default or `-> ()`. Non-unit returns need a fallback value for
//!   the panicked case — return-type-dependent, not parse-only provable.
//!   Suggest-only territory (the corpus fixture shows the unit shape).
//! - **Already-contained skip**: the fn body text containing
//!   `catch_unwind` anywhere means containment is already handled — skip.
//! - **No caller gate needed**: the wrap preserves the signature verbatim.
//!
//! ## Honest caveats
//! - The closure must be `UnwindSafe`: bodies capturing `&mut` or guards
//!   (`MutexGuard`) fail to compile — the `--fix --verify` compile gate
//!   reverts those. Parse-only cannot prove `UnwindSafe`.
//! - Panics are SWALLOWED (result discarded). The in-house macro maps to an
//!   error code; a span-local rewrite cannot invent one for arbitrary
//!   signatures — the unit-return scope is exactly the case where
//!   swallowing is the contract.

use syn::spanned::Spanned;
use syn::{Block, File, ReturnType, Signature};

use super::{line_indent, src, SourceEdit};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    walk_fns(&file.items, source, &mut edits);
    edits
}

fn walk_fns(items: &[syn::Item], source: &str, edits: &mut Vec<SourceEdit>) {
    for item in items {
        match item {
            syn::Item::Fn(f) => visit_fn(&f.sig, &f.block, source, edits),
            syn::Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    walk_fns(inner, source, edits);
                }
            }
            syn::Item::Impl(imp) => {
                for ii in &imp.items {
                    if let syn::ImplItem::Fn(m) = ii {
                        visit_fn(&m.sig, &m.block, source, edits);
                    }
                }
            }
            _ => {}
        }
    }
}

fn unit_return(output: &ReturnType) -> bool {
    match output {
        ReturnType::Default => true,
        ReturnType::Type(_, ty) => {
            matches!(ty.as_ref(), syn::Type::Tuple(t) if t.elems.is_empty())
        }
    }
}

fn visit_fn(sig: &Signature, block: &Block, source: &str, edits: &mut Vec<SourceEdit>) {
    // extern ABI + unit return only (see module docs).
    if sig.abi.is_none() || !unit_return(&sig.output) {
        return;
    }
    let Some(block_text) = src(source, block) else {
        return;
    };
    // Already contained — skip (double-wrap is worse than useless).
    if block_text.contains("catch_unwind") {
        return;
    }
    let r = block.span().byte_range();
    let indent = line_indent(source, r.start);
    // `{` newline indent `let _ = std::panic::catch_unwind(|| <block>);` `}` —
    // the original block nests VERBATIM as the closure body (a block is an
    // expression; its interior formatting is untouched).
    let replacement =
        format!("{{\n{indent}    let _ = std::panic::catch_unwind(|| {block_text});\n{indent}}}");
    edits.push(SourceEdit {
        start: r.start,
        end: r.end,
        replacement,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_edit(src_text: &str) -> SourceEdit {
        let file = syn::parse_file(src_text).expect("parse");
        let edits = collect_edits(src_text, &file);
        assert_eq!(edits.len(), 1, "want exactly one edit");
        edits.into_iter().next().expect("one")
    }

    #[test]
    fn wraps_unit_return_extern_fn() {
        let e = one_edit(
            "#[no_mangle]\npub extern \"C\" fn on_packet(n: u32) {\n    let v = vec![n; 4];\n    println!(\"{v:?}\");\n}\n",
        );
        let out = super::super::apply_edits(
            "#[no_mangle]\npub extern \"C\" fn on_packet(n: u32) {\n    let v = vec![n; 4];\n    println!(\"{v:?}\");\n}\n",
            vec![e],
        );
        assert!(out.contains("catch_unwind(|| {"), "wrapped: {out}");
        // The body nests verbatim.
        assert!(out.contains("let v = vec![n; 4];"));
    }

    #[test]
    fn skips_non_extern_fn() {
        let src = "pub fn plain(n: u32) { println!(\"{n}\"); }";
        let file = syn::parse_file(src).unwrap();
        assert_eq!(
            collect_edits(src, &file),
            [] as [crate::edit_emitter::SourceEdit; 0]
        );
    }

    #[test]
    fn skips_non_unit_return() {
        let src = "pub extern \"C\" fn f(n: u32) -> u32 { n + 1 }";
        let file = syn::parse_file(src).unwrap();
        assert_eq!(
            collect_edits(src, &file),
            [] as [crate::edit_emitter::SourceEdit; 0]
        );
    }

    #[test]
    fn skips_already_contained() {
        let src = "pub extern \"C\" fn f() {\n    let _ = std::panic::catch_unwind(|| t());\n}";
        let file = syn::parse_file(src).unwrap();
        assert_eq!(
            collect_edits(src, &file),
            [] as [crate::edit_emitter::SourceEdit; 0]
        );
    }

    #[test]
    fn fires_on_explicit_unit_and_other_abis() {
        let src = "pub extern \"system\" fn f() -> () { work(); }";
        let file = syn::parse_file(src).unwrap();
        assert_eq!(collect_edits(src, &file).len(), 1);
    }
}

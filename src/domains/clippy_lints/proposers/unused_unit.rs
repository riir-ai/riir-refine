//! `clippy::unused_unit` — `fn f() -> () {}` → `fn f() {}` (Issue 016).
//!
//! Pattern: a fn signature whose output type is the empty tuple. The
//! `-> ()` is deleted; when a space precedes the arrow it is swallowed
//! too, so no double space remains. Covers free fns, impl/trait methods,
//! and closures' enclosing fns alike via `visit_signature`.

use crate::edit_emitter::span_range;
use super::SourceEdit;
use syn::visit::Visit;
use syn::{File, ReturnType, Signature, Type};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector { source, edits: Vec::new() };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_signature(&mut self, sig: &'ast Signature) {
        syn::visit::visit_signature(self, sig);
        let ReturnType::Type(_, ty) = &sig.output else {
            return;
        };
        let Type::Tuple(t) = ty.as_ref() else {
            return;
        };
        if !t.elems.is_empty() {
            return; // `(u8, u8)` is a real tuple, not unit
        }
        let (mut start, end) = span_range(&sig.output);
        if start > 0 && self.source[..start].ends_with(' ') {
            start -= 1; // `fn f() -> ()` → `fn f()` — not `fn f(> )`
        }
        self.edits.push(SourceEdit {
            start,
            end,
            replacement: String::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn free_fn_unit_return_stripped() {
        let out =
            propose_fix("unused_unit", "fn f() -> () { let _ = 1; }").expect("pattern matched");
        assert_eq!(out, "fn f() { let _ = 1; }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn impl_method_unit_return_stripped() {
        let input = "struct S;\nimpl S {\n    fn m(&self) -> () {}\n}";
        let out = propose_fix("unused_unit", input).expect("pattern matched");
        assert!(out.contains("fn m(&self) {}"), "{out}");
        assert!(out.contains("struct S;"), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn no_return_annotation_unchanged() {
        let out = propose_fix("unused_unit", "fn f() { let _ = 1; }");
        assert!(out.is_none(), "already unit-implicit changed: {out:?}");
    }

    #[test]
    fn concrete_return_unchanged() {
        let out = propose_fix("unused_unit", "fn f() -> u8 { 1 }");
        assert!(out.is_none(), "`-> u8` must not be touched: {out:?}");
    }

    #[test]
    fn real_tuple_return_unchanged() {
        let out = propose_fix("unused_unit", "fn f() -> (u8, u8) { (1, 2) }");
        assert!(out.is_none(), "non-empty tuple is not unit: {out:?}");
    }
}

//! `clippy::manual_string_new` — `String::from("")` → `String::new()`
//! (Batch 163 / Plan 154 — the frontier-lead mining pack IX; the family
//! fixture verified to fire exactly this lint under all+pedantic+nursery
//! with a clean seed, 2026-09-19 (`scripts/plan154_frontier_probe.py`)).
//!
//! ## Why the rewrite is construction identity (the FIXABLE bar)
//!
//! `String::from("")` is `impl From<&str> for String` = `"".to_owned()` =
//! an empty `String` over `Vec::with_capacity(0)` — which does not
//! allocate. `String::new()` is the same value: empty, zero capacity, no
//! allocation, no side effects on either side. The rewrite mirrors clippy's
//! own `MachineApplicable` suggestion (probe-measured: the rendered
//! replacement is exactly `String::new()`), the same
//! suggestion-shape-verbatim posture as Batch 161's `unit_arg`.
//!
//! ## Guards (the bounded fix-space)
//!
//! - The callee must be a 2-segment path `String::from`, or the explicit
//!   turbofish `<String as From<..>>::from` (the probe: clippy fires BOTH —
//!   the turbofish re-spells the type the path already names, so the
//!   rewrite is resolution-identical in both spellings). `From::from("")`
//!   (inference could be `Cow`/`Box`/any `From<&str>`) and
//!   `std::string::String::from` (longer paths) refuse.
//! - The single argument must be the EMPTY string literal (probe:
//!   `String::from("x")`, `String::from(s)`, and even
//!   `let s = ""; String::from(s)` are all SILENT — clippy is
//!   literal-arg-only). A non-empty literal, a variable, a macro, any other
//!   arg shape refuses.
//! - `"".to_string()` FIRES upstream (probe) but refuses here: the rewrite
//!   seam is a method call, not a path call, and our v1 scope is the
//!   from-call family — a documented narrower-than-clippy divergence (the
//!   SAFER-than-clippy class).
//! - Attributes or comments inside the replaced call span refuse (the span
//!   re-spells to `String::new()`).

use syn::visit::Visit;
use syn::{Expr, ExprCall, File, Lit};

use super::helpers::byte_range;
use super::{edit, SourceEdit};
use crate::edit_emitter::drops_interior_comment;

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

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_call(&mut self, c: &'ast ExprCall) {
        syn::visit::visit_expr_call(self, c);
        if let Some(replacement) = self.try_rewrite(c) {
            self.edits.push(edit(c, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_rewrite(&mut self, c: &ExprCall) -> Option<String> {
        if c.args.len() != 1 || !c.attrs.is_empty() {
            return None;
        }
        if !is_empty_str_lit(&c.args[0]) {
            return None;
        }
        let Expr::Path(p) = &*c.func else { return None };
        if is_string_from_path(p).is_err() {
            return None;
        }
        if drops_interior_comment(self.source, byte_range(c), &[]) {
            return None;
        }
        Some("String::new()".to_string())
    }
}

/// `Ok(())` for `String::from` (2 segments, no qself) or the explicit
/// turbofish `<String as From<..>>::from`; `Err(())` for every other shape
/// (`From::from`, longer paths, method syntax, generic segments).
fn is_string_from_path(p: &syn::ExprPath) -> Result<(), ()> {
    if !p.attrs.is_empty()
        || p.path.leading_colon.is_some()
        || p.path.segments.len() != 2
        || p.path.segments[1].ident != "from"
        || !p.path.segments[1].arguments.is_none()
    {
        return Err(());
    }
    let a = &p.path.segments[0];
    // Plain `String::from` — no qself, no generic args on `String`.
    if p.qself.is_none() {
        if a.ident == "String" && a.arguments.is_none() {
            return Ok(());
        }
        return Err(());
    }
    // Explicit turbofish `<String as From<..>>::from` — qself names the
    // type; the trait segment is `From` (its angle args are whatever the
    // original spelled — the rewrite drops the whole qualified path).
    let q = p.qself.as_ref().ok_or(())?;
    if a.ident != "From" {
        return Err(());
    }
    let syn::Type::Path(tp) = &*q.ty else {
        return Err(());
    };
    if tp.qself.is_none()
        && tp.path.leading_colon.is_none()
        && tp.path.segments.len() == 1
        && tp.path.segments[0].ident == "String"
        && tp.path.segments[0].arguments.is_none()
    {
        return Ok(());
    }
    Err(())
}

/// Exactly the empty string literal — a variable, non-empty literal, macro,
/// or any other arg shape refuses (clippy is literal-arg-only, probe-pinned).
fn is_empty_str_lit(e: &Expr) -> bool {
    match e {
        Expr::Lit(l) => matches!(&l.lit, Lit::Str(s) if s.value().is_empty()),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            "manual_string_new",
            "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::new() }",
        )
        .is_none());
    }

    #[test]
    fn from_empty_fires() {
        let out = propose_fix(
            "manual_string_new",
            "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::from(\"\") }",
        )
        .expect("matched");
        assert!(out.contains("String::new()"), "not rewritten: {out}");
        assert!(!out.contains("from("), "call not collapsed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn turbofish_fires() {
        // <String as From<&str>>::from("") — clippy fires (probe); the
        // rewrite is resolution-identical.
        let out = propose_fix(
            "manual_string_new",
            "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); <String as From<&str>>::from(\"\") }",
        )
        .expect("matched");
        assert!(out.contains("String::new()"), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_in_larger_expr_fires() {
        let out = propose_fix(
            "manual_string_new",
            "fn mk(hits: &mut Vec<u8>) -> Option<String> { hits.push(0); Some(String::from(\"\")) }",
        )
        .expect("matched");
        assert!(out.contains("Some(String::new())"), "not in place: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn non_empty_literal_refuses() {
        assert!(propose_fix(
            "manual_string_new",
            "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::from(\"x\") }",
        )
        .is_none());
    }

    #[test]
    fn variable_arg_refuses() {
        // clippy is literal-arg-only (probe: String::from(s) SILENT).
        assert!(propose_fix(
            "manual_string_new",
            "fn mk(s: &str, hits: &mut Vec<u8>) -> String { hits.push(0); String::from(s) }",
        )
        .is_none());
    }

    #[test]
    fn inferred_from_refuses() {
        // `From::from("")` could be Cow/Box/any From<&str> — not the family.
        assert!(propose_fix(
            "manual_string_new",
            "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); From::from(\"\") }",
        )
        .is_none());
    }

    #[test]
    fn long_path_refuses() {
        assert!(propose_fix(
            "manual_string_new",
            "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); std::string::String::from(\"\") }",
        )
        .is_none());
    }

    #[test]
    fn to_string_shape_refuses() {
        // Fires upstream (probe) but the seam is a method call — the
        // documented narrower-than-clippy divergence for v1.
        assert!(propose_fix(
            "manual_string_new",
            "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); \"\".to_string() }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            "manual_string_new",
            "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::from(/* why */ \"\") }",
        )
        .is_none());
    }
}

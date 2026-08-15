//! `clippy::redundant_static_lifetimes` — `static S: &'static str = "x";`
//! → `static S: &str = "x";` (same for `const`, Issue 016).
//!
//! In `static`/`const` items a reference type defaults to `'static`, so
//! the annotation is redundant. Span-preserving: the `'static` token is
//! deleted together with ONE following space (`&'static str` → `&str`).

use super::{edit, swallow_following_space, SourceEdit};
use syn::visit::Visit;
use syn::{File, ItemConst, ItemStatic, Type};

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
    fn visit_item_static(&mut self, s: &'ast ItemStatic) {
        syn::visit::visit_item_static(self, s);
        self.check(&s.ty);
    }

    fn visit_item_const(&mut self, c: &'ast ItemConst) {
        syn::visit::visit_item_const(self, c);
        self.check(&c.ty);
    }
}

impl Collector<'_> {
    /// `&'static T` in a static/const item type → delete the lifetime.
    fn check(&mut self, ty: &Type) {
        let Type::Reference(r) = ty else { return };
        let Some(lt) = &r.lifetime else { return };
        if lt.ident != "static" {
            return;
        }
        self.edits
            .push(swallow_following_space(self.source, edit(lt, String::new())));
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn static_str_lifetime_stripped() {
        let out = propose_fix(
            "redundant_static_lifetimes",
            "static S: &'static str = \"x\";",
        )
        .expect("pattern matched");
        assert_eq!(out, "static S: &str = \"x\";");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn const_slice_lifetime_stripped() {
        let out =
            propose_fix("redundant_static_lifetimes", "const C: &'static [u8] = b\"x\";")
                .expect("pattern matched");
        assert_eq!(out, "const C: &[u8] = b\"x\";");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn already_elided_unchanged() {
        let out = propose_fix("redundant_static_lifetimes", "static S: &str = \"x\";");
        assert!(out.is_none(), "elided lifetime changed: {out:?}");
    }

    #[test]
    fn non_static_lifetime_unchanged() {
        let out = propose_fix(
            "redundant_static_lifetimes",
            "fn f<'a>(x: &'a u8) -> &'a u8 { x }",
        );
        assert!(out.is_none(), "'a is not redundant: {out:?}");
    }
}

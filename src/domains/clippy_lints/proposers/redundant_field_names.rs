//! `clippy::redundant_field_names` — `Foo { x: x, y: y }` → `Foo { x, y }`.
//!
//! Pattern: `ExprStruct` whose fields have `name: value` where `value` is an
//! `Expr::Path` with a single ident equal to `name`.
//!
//! Span-preserving (T6.4): each redundant field's byte range (`x: x`) is
//! replaced by the member name alone. Fields carrying attributes are
//! skipped — the replacement would drop them.

use super::{edit, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprStruct, File};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(_source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector { edits: Vec::new() };
    v.visit_file(file);
    v.edits
}

struct Collector {
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_expr_struct(&mut self, s: &'ast ExprStruct) {
        syn::visit::visit_expr_struct(self, s);
        for field in &s.fields {
            if is_redundant(field) {
                // The field span covers `name : value` (attrs are guarded
                // out); the shorthand keeps just the member ident.
                let syn::Member::Named(ident) = &field.member else {
                    continue;
                };
                self.edits.push(edit(field, ident.to_string()));
            }
        }
    }
}

/// Is this field a redundant `name: name` (member == value-ident)?
fn is_redundant(field: &syn::FieldValue) -> bool {
    if field.colon_token.is_none() || !field.attrs.is_empty() {
        return false; // already shorthand / carries attributes
    }
    let syn::Member::Named(member_ident) = &field.member else {
        return false;
    };
    let val_ident = match &field.expr {
        Expr::Path(p) => p.path.get_ident(),
        _ => return false,
    };
    match val_ident {
        Some(id) => id == member_ident,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn makes_redundant_fields_shorthand() {
        let input = "struct Cfg { name: String, port: u16 }\nfn mk(name: String, port: u16) -> Cfg {\n    Cfg { name: name, port: port }\n}";
        let out = propose_fix("redundant_field_names", input).expect("matched");
        assert!(out.contains("Cfg { name, port }"), "shorthand not produced: {out}");
        assert!(out.contains("struct Cfg { name: String, port: u16 }"), "struct def must be untouched: {out}");
    }

    #[test]
    fn already_shorthand_unchanged() {
        let input = "struct Cfg { name: String }\nfn mk(name: String) -> Cfg { Cfg { name } }";
        let out = propose_fix("redundant_field_names", input);
        assert!(out.is_none(), "already-shorthand changed: {out:?}");
    }

    #[test]
    fn attributed_field_unchanged() {
        // Attributes would be lost by the splice — skip conservatively.
        let input = "struct C { x: u32 }\nfn mk(x: u32) -> C { C { #[cfg(x)] x: x } }";
        let out = propose_fix("redundant_field_names", input);
        assert!(out.is_none(), "attributed field must not be touched: {out:?}");
    }

    #[test]
    fn differing_name_value_unchanged() {
        let input = "struct C { x: u32, y: u32 }\nfn mk(x: u32, y: u32) -> C { C { x: y, y: x } }";
        let out = propose_fix("redundant_field_names", input);
        assert!(out.is_none(), "non-redundant fields touched: {out:?}");
    }
}

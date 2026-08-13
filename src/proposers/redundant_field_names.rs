//! `clippy::redundant_field_names` — `Foo { x: x, y: y }` → `Foo { x, y }`.
//!
//! Pattern: `ExprStruct` whose fields have `name: value` where `value` is an
//! `Expr::Path` with a single ident equal to `name`.

use syn::{Expr, ExprStruct, File};

pub(crate) fn rewrite(file: &mut File) -> bool {
    use syn::visit_mut::VisitMut;
    struct Visitor {
        changed: bool,
    }
    impl VisitMut for Visitor {
        fn visit_expr_mut(&mut self, e: &mut Expr) {
            syn::visit_mut::visit_expr_mut(self, e);
            if let Expr::Struct(s) = e {
                self.changed |= shorten_struct_fields(s);
            }
        }
    }
    let mut v = Visitor { changed: false };
    syn::visit_mut::visit_file_mut(&mut v, file);
    v.changed
}

/// Walk a struct literal's fields, setting `name: name` to shorthand `name`.
fn shorten_struct_fields(s: &mut ExprStruct) -> bool {
    let mut changed = false;
    for field in &mut s.fields {
        if is_redundant(field) {
            // In syn 2.x, clearing `colon_token` makes the field render as
            // shorthand (only `member` is emitted; `expr` is ignored).
            field.colon_token = None;
            changed = true;
        }
    }
    changed
}

/// Is this field a redundant `name: name` (member == value-ident)?
fn is_redundant(field: &syn::FieldValue) -> bool {
    if field.colon_token.is_none() {
        return false; // already shorthand
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
    use super::*;

    fn propose(input: &str) -> Option<String> {
        let mut file = syn::parse_file(input).ok()?;
        if rewrite(&mut file) {
            Some(crate::proposers::quote_file(&file))
        } else {
            None
        }
    }

    #[test]
    fn makes_redundant_fields_shorthand() {
        let input = "struct Cfg { name: String, port: u16 }\nfn mk(name: String, port: u16) -> Cfg {\n    Cfg { name: name, port: port }\n}";
        let out = propose(input).expect("matched");
        // After token-stream round-trip, the struct literal should print as
        // `Cfg { name , port }` (no `: name`).
        assert!(
            out.contains("Cfg { name , port }") || out.contains("Cfg { name, port }"),
            "shorthand not produced: {out}"
        );
    }

    #[test]
    fn already_shorthand_unchanged() {
        let input = "struct Cfg { name: String }\nfn mk(name: String) -> Cfg { Cfg { name } }";
        let out = propose(input);
        assert!(out.is_none(), "already-shorthand changed: {out:?}");
    }
}

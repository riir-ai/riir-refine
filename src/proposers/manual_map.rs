//! `clippy::manual_map` — `match opt { Some(v) => Some(f(v)), None => None }` →
//! `opt.map(|v| f(v))`.

use syn::{Arm, Expr, ExprMatch, File, Pat};

pub(crate) fn rewrite(file: &mut File) -> bool {
    use syn::visit_mut::VisitMut;
    struct Visitor {
        changed: bool,
    }
    impl VisitMut for Visitor {
        fn visit_expr_mut(&mut self, e: &mut Expr) {
            syn::visit_mut::visit_expr_mut(self, e);
            if let Expr::Match(m) = e {
                if let Some(replacement) = try_manual_map(m) {
                    *e = replacement;
                    self.changed = true;
                }
            }
        }
    }
    let mut v = Visitor { changed: false };
    syn::visit_mut::visit_file_mut(&mut v, file);
    v.changed
}

fn try_manual_map(m: &mut ExprMatch) -> Option<Expr> {
    if m.arms.len() != 2 {
        return None;
    }
    // Find which arm is the Some + which is the None.
    let (some_idx, none_idx) = match (
        is_some_pat(&m.arms[0].pat),
        is_none_pat(&m.arms[1].pat),
    ) {
        (true, true) => (0usize, 1usize),
        _ => match (
            is_none_pat(&m.arms[0].pat),
            is_some_pat(&m.arms[1].pat),
        ) {
            (true, true) => (1usize, 0usize),
            _ => return None,
        },
    };
    // none arm body must be the bare `None`.
    if !is_none_expr(&m.arms[none_idx].body) {
        return None;
    }
    // some arm body must be `Some(<expr>)`.
    let some_arm = &m.arms[some_idx];
    let inner: Expr = match &*some_arm.body {
        Expr::Call(c) => {
            if !matches!(&*c.func, Expr::Path(p) if p.path.is_ident("Some")) {
                return None;
            }
            if c.args.len() != 1 {
                return None;
            }
            c.args[0].clone()
        }
        _ => return None,
    };
    // some arm pat must be `Some(<binding>)`.
    let binding = match &some_arm.pat {
        Pat::TupleStruct(ts) if ts.path.is_ident("Some") && ts.elems.len() == 1 => {
            match &ts.elems[0] {
                Pat::Ident(pi) => pi.clone(),
                _ => return None,
            }
        }
        _ => return None,
    };
    let scrutinee = &m.expr;
    let map_call: Expr = syn::parse_quote!(#scrutinee . map ( | #binding | #inner ));
    Some(map_call)
}

fn is_some_pat(p: &Pat) -> bool {
    matches!(p, Pat::TupleStruct(ts) if ts.path.is_ident("Some") && ts.elems.len() == 1)
}

fn is_none_pat(p: &Pat) -> bool {
    // `None` can parse as Pat::Path (when treated as a unit-variant path),
    // Pat::TupleStruct(empty) (when treated as a unit-struct pattern), or
    // Pat::Ident (syn ambiguity — `None` is also a valid binding name).
    // We check the token text to disambiguate.
    matches!(p, Pat::Path(pp) if pp.path.is_ident("None"))
        || matches!(p, Pat::TupleStruct(ts) if ts.path.is_ident("None") && ts.elems.is_empty())
        || matches!(p, Pat::Ident(pi) if pi.ident == "None")
}

fn is_none_expr(e: &Expr) -> bool {
    matches!(e, Expr::Path(pp) if pp.path.is_ident("None"))
}

#[allow(dead_code)]
fn _silence_arm_unused(_a: &Arm) {}

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
    fn converts_match_to_map() {
        let input = "fn to_text(x: Option<u8>) -> Option<String> {\n    match x {\n        Some(v) => Some(v.to_string()),\n        None => None,\n    }\n}";
        let out = propose(input).expect("matched");
        assert!(out.contains("map"), "map not produced: {out}");
    }

    #[test]
    fn leaves_non_match_unchanged() {
        let input = "fn to_text(x: Option<u8>) -> Option<String> { x.map(|v| v.to_string()) }";
        let out = propose(input);
        assert!(out.is_none(), "non-match touched: {out:?}");
    }

    #[test]
    fn leaves_two_arm_match_not_some_none() {
        let input = "fn go(x: u8) -> u8 { match x { 1 => 2, _ => 3 } }";
        let out = propose(input);
        assert!(out.is_none(), "non-manual-map match touched: {out:?}");
    }
}

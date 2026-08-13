//! `clippy::needless_range_loop` — `for i in 0..v.len() { body_using(v[i]) }`
//! → `for x in &v { body_using_substituted }`.

use syn::{Expr, ExprForLoop, File};

pub(crate) fn rewrite(file: &mut File) -> bool {
    use syn::visit_mut::VisitMut;
    struct Visitor {
        changed: bool,
    }
    impl VisitMut for Visitor {
        fn visit_expr_mut(&mut self, e: &mut Expr) {
            syn::visit_mut::visit_expr_mut(self, e);
            if let Expr::ForLoop(fl) = e {
                if let Some(replacement) = try_rewrite_for(fl) {
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

fn try_rewrite_for(fl: &ExprForLoop) -> Option<Expr> {
    let Expr::Range(range) = &*fl.expr else {
        return None;
    };
    check_range_start_zero(range)?;
    let collection_path = check_range_end_v_len(range)?;

    let idx_ident = match &*fl.pat {
        syn::Pat::Ident(pi) => &pi.ident,
        _ => return None,
    };

    let body: &syn::Block = &fl.body;
    let usage = analyze_index_usage(body, &collection_path, idx_ident);

    if !usage.idx_used_only_for_collection_index {
        return None;
    }
    if usage.index_count == 0 {
        return None;
    }

    // Clone the inner `Block` so we can mutate it.
    let mut new_body: syn::Block = body.clone();
    let fresh = syn::Ident::new("__elem", proc_macro2::Span::call_site());
    substitute_indexing(&mut new_body, &collection_path, idx_ident, &fresh);

    let replacement: Expr = syn::parse_quote!(for #fresh in & #collection_path { #new_body });
    Some(replacement)
}

fn check_range_start_zero(range: &syn::ExprRange) -> Option<()> {
    let start = range.start.as_deref()?;
    if matches!(start, Expr::Lit(el) if is_int_literal_zero(el)) {
        Some(())
    } else {
        None
    }
}

fn check_range_end_v_len(range: &syn::ExprRange) -> Option<syn::ExprPath> {
    let end = range.end.as_deref()?;
    let end_call = match end {
        Expr::MethodCall(mc) if mc.method == "len" && mc.args.is_empty() => &*mc.receiver,
        _ => return None,
    };
    if let Expr::Path(p) = end_call {
        Some(p.clone())
    } else {
        None
    }
}

fn is_int_literal_zero(el: &syn::ExprLit) -> bool {
    if let syn::Lit::Int(li) = &el.lit {
        li.base10_digits() == "0"
    } else {
        false
    }
}

struct IndexUsage {
    index_count: usize,
    idx_used_only_for_collection_index: bool,
}

fn analyze_index_usage(
    block: &syn::Block,
    collection: &syn::ExprPath,
    idx: &syn::Ident,
) -> IndexUsage {
    use quote::ToTokens;
    let mut tokens = proc_macro2::TokenStream::new();
    block.to_tokens(&mut tokens);
    let s = tokens.to_string();

    let coll_name = collection
        .path
        .get_ident()
        .map(std::string::ToString::to_string)
        .unwrap_or_default();
    let idx_name = idx.to_string();

    let mut index_count = 0usize;
    let normalized = s.replace(", ", ",").replace(" ,", ",");
    let needle_owned = format!("{coll_name} [ {idx_name} ]");
    let needle_alt = format!("{coll_name} [{idx_name}]");
    let mut start = 0;
    while let Some(off) = normalized[start..].find(&needle_owned) {
        index_count += 1;
        start += off + needle_owned.len();
    }
    let mut start = 0;
    while let Some(off) = normalized[start..].find(&needle_alt) {
        index_count += 1;
        start += off + needle_alt.len();
    }

    let idx_count = s.matches(&idx_name).count();
    let idx_used_only_for_collection_index = idx_count == index_count;

    IndexUsage {
        index_count,
        idx_used_only_for_collection_index,
    }
}

fn substitute_indexing(
    block: &mut syn::Block,
    collection: &syn::ExprPath,
    idx: &syn::Ident,
    fresh: &syn::Ident,
) {
    struct Sub {
        coll_name: String,
        idx_name: String,
        fresh: syn::Ident,
    }
    impl syn::visit_mut::VisitMut for Sub {
        fn visit_expr_mut(&mut self, e: &mut Expr) {
            syn::visit_mut::visit_expr_mut(self, e);
            if let Expr::Index(ix) = e {
                let coll_match = matches!(&*ix.expr, Expr::Path(p) if p.path.is_ident(&self.coll_name));
                let idx_match = matches!(&*ix.index, Expr::Path(p) if p.path.is_ident(&self.idx_name));
                if coll_match && idx_match {
                    let fresh = self.fresh.clone();
                    *e = Expr::Path(syn::ExprPath {
                        attrs: Vec::new(),
                        qself: None,
                        path: fresh.into(),
                    });
                }
            }
        }
    }
    let coll_name = collection
        .path
        .get_ident()
        .map(std::string::ToString::to_string)
        .unwrap_or_default();
    let mut sub = Sub {
        coll_name,
        idx_name: idx.to_string(),
        fresh: fresh.clone(),
    };
    for stmt in &mut block.stmts {
        syn::visit_mut::visit_stmt_mut(&mut sub, stmt);
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
    fn converts_range_loop_to_iter_loop() {
        let input = "fn double_all(v: &Vec<i32>) -> Vec<i32> {\n    let mut out = Vec::new();\n    for i in 0..v.len() { out.push(v[i] * 2); }\n    out\n}";
        let out = propose(input).expect("matched");
        assert!(out.contains("& v"), "iter not produced: {out}");
        assert!(!out.contains("0 .."), "range not removed: {out}");
    }

    #[test]
    fn leaves_non_range_loop_unchanged() {
        let input = "fn go(v: Vec<i32>) { for x in v { println!(\"{}\", x); } }";
        let out = propose(input);
        assert!(out.is_none(), "non-range loop touched: {out:?}");
    }

    #[test]
    fn leaves_loop_with_arithmetic_index() {
        let input = "fn go(v: Vec<i32>) { for i in 0..v.len() { println!(\"{}\", v[i] + i); } }";
        let out = propose(input);
        assert!(out.is_none(), "arithmetic-index loop touched: {out:?}");
    }
}

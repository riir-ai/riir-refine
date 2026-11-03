//! `clippy::used_underscore_binding` — a binding named `_x` that IS used
//! renames to `x` at its binding site and every use site (P51a, 2026-10-14;
//! pedantic group per the pinned 1.98.1 driver; upstream is DIAGNOSIS-ONLY
//! in 1.98.1 — no suggestion — the rename is OUR rewrite, the multi-span
//! P15 structural class).
//!
//! ## The rewrite
//!
//! Per FUNCTION (free fns and impl methods — a scope stack groups bindings
//! and uses to their innermost fn): for every binding `_name` (leading `_`,
//! next char alphabetic — dunder and plain-`_` forms are not candidates)
//! that has at least one PATH USE in the SAME fn, emit one edit per
//! occurrence of the ident in that fn — binding patterns AND use sites
//! strip the leading underscore.
//!
//! ## Guards (the multi-span safety legs)
//!
//! - **Unused never renames**: the underscore on an unused binding is
//!   load-bearing (it suppresses the unused warning) — renaming would
//!   BIRTH a diagnostic, the anti-goal. Per-fn grouping carries this
//!   across fns too: a `_x` unused in fn B stays `_x` even when fn A
//!   renames its own used `_x`.
//! - **Uses-without-bindings never rename**: a `_x` path use whose binding
//!   is not an in-fn pattern (a module const/static — upstream's separate
//!   `used_underscore_items` shape) refuses; the binding site is where the
//!   rename must start.
//! - **File-level collision guard**: if the target name `x` appears
//!   anywhere in the file as a pattern ident, a path ident, or an item
//!   name (fn/struct/enum/mod/const/static), the rename refuses — a
//!   same-file `x` could capture the renamed uses.
//! - **Macro-token guard**: if `_x`'s text appears inside ANY macro token
//!   stream, the rename refuses — syn cannot see macro-internal uses, so
//!   a rename would strand a macro-internal `_x` reference (E0425).
//! - **Shorthand guard**: `_x` in a struct-shorthand PATTERN (`S { _x }`)
//!   or EXPRESSION (`S { _x }`) refuses — the member name and the binding
//!   would have to diverge, which a byte-splice cannot express (the
//!   `manual_let_else` `Inject { raw_sig }` lesson).
//!
//! ## Why the rename is safe
//!
//! Renaming a used binding changes no resolution: every occurrence of the
//! ident text in the fn renames together, the census proves no target-name
//! collision exists in the file, and shadowing among multiple `_x`
//! bindings in one fn is preserved (all become `x`, same topology). The
//! underscore prefix on a USED binding is warning-suppression metadata
//! only — dead weight, which is the lint's point.

use syn::visit::Visit;
use syn::{Expr, File, ItemFn, Pat};

use super::SourceEdit;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(_source: &str, file: &File) -> Vec<SourceEdit> {
    let mut census = Census::default();
    census.visit_file(file);
    let mut edits = Vec::new();
    for cand in census.finished.iter().flatten() {
        // Used in THIS fn, bound in THIS fn, no file-level conflicts.
        if cand.bindings.is_empty() || cand.path_uses.is_empty() {
            continue;
        }
        if census.names.contains(&cand.target) {
            continue;
        }
        if census.macro_hits.contains(&cand.name) {
            continue;
        }
        if census.shorthand_hits.contains(&cand.name) {
            continue;
        }
        for r in cand.bindings.iter().chain(cand.path_uses.iter()) {
            edits.push(SourceEdit {
                start: r.start,
                end: r.end,
                replacement: cand.target.clone(),
            });
        }
    }
    edits.shrink_to_fit();
    edits
}

/// One fn's rename candidate for `_name`: stripped `target`, binding
/// spans, path-use spans (all within one fn scope).
struct Candidate {
    name: String,
    target: String,
    bindings: Vec<std::ops::Range<usize>>,
    path_uses: Vec<std::ops::Range<usize>>,
}

/// The file-wide census: per-fn candidate stacks + the file-level guards.
#[derive(Default)]
struct Census {
    /// Completed fn scopes (the stack pops into here — the candidates
    /// survive the traversal).
    finished: Vec<Vec<Candidate>>,
    /// One candidate vec per fn on the scope stack.
    scopes: Vec<Vec<Candidate>>,
    /// Every plain (non-candidate) lowercase ident in patterns, paths,
    /// and item names — the collision set.
    names: std::collections::BTreeSet<String>,
    /// Ident texts seen inside macro token streams.
    macro_hits: std::collections::BTreeSet<String>,
    /// Ident texts seen in struct-shorthand positions.
    shorthand_hits: std::collections::BTreeSet<String>,
}

/// Is this ident a rename candidate head? `_x` with an alphabetic tail.
fn is_candidate_name(name: &str) -> bool {
    name.starts_with('_') && name.chars().nth(1).is_some_and(|c| c.is_ascii_alphabetic())
}

impl Census {
    fn record_binding(&mut self, name: &str, range: std::ops::Range<usize>) {
        let Some(scope) = self.scopes.last_mut() else {
            return; // a pattern outside any fn (const closures, trait
                    // default bodies) — not our shape, silently skipped
        };
        match scope.iter_mut().find(|c| c.name == name) {
            Some(c) => c.bindings.push(range),
            None => scope.push(Candidate {
                name: name.to_string(),
                target: name[1..].to_string(),
                bindings: vec![range],
                path_uses: Vec::new(),
            }),
        }
    }

    fn record_use(&mut self, name: &str, range: std::ops::Range<usize>) {
        let Some(scope) = self.scopes.last_mut() else {
            return;
        };
        match scope.iter_mut().find(|c| c.name == name) {
            Some(c) => c.path_uses.push(range),
            None => scope.push(Candidate {
                name: name.to_string(),
                target: name[1..].to_string(),
                bindings: Vec::new(),
                path_uses: vec![range],
            }),
        }
    }
}

impl<'ast> Visit<'ast> for Census {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        self.scopes.push(Vec::new());
        syn::visit::visit_item_fn(self, i);
        // The fn's name joins the collision set (a local `x` would shadow
        // it for unqualified path resolution).
        self.names.insert(i.sig.ident.to_string());
        if let Some(done) = self.scopes.pop() {
            self.finished.push(done);
        }
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        self.scopes.push(Vec::new());
        syn::visit::visit_impl_item_fn(self, i);
        self.names.insert(i.sig.ident.to_string());
        if let Some(done) = self.scopes.pop() {
            self.finished.push(done);
        }
    }

    fn visit_pat(&mut self, p: &'ast Pat) {
        if let Pat::Struct(ps) = p {
            for f in &ps.fields {
                if f.colon_token.is_none() {
                    if let Pat::Ident(pi) = &*f.pat {
                        self.shorthand_hits.insert(pi.ident.to_string());
                    }
                }
            }
        }
        if let Pat::Ident(pi) = p {
            let name = pi.ident.to_string();
            if is_candidate_name(&name) {
                self.record_binding(&name, pi.ident.span().byte_range());
            } else if !name.chars().next().is_some_and(char::is_uppercase) && name != "_" {
                self.names.insert(name);
            }
        }
        syn::visit::visit_pat(self, p);
    }

    fn visit_expr(&mut self, e: &'ast Expr) {
        if let Expr::Struct(es) = e {
            for f in &es.fields {
                if f.colon_token.is_none() {
                    if let syn::Member::Named(id) = &f.member {
                        self.shorthand_hits.insert(id.to_string());
                    }
                }
            }
        }
        if let Expr::Path(ep) = e {
            if ep.path.segments.len() == 1 && ep.qself.is_none() {
                let seg = &ep.path.segments[0];
                let name = seg.ident.to_string();
                if is_candidate_name(&name) {
                    self.record_use(&name, seg.ident.span().byte_range());
                } else if !name.chars().next().is_some_and(char::is_uppercase) {
                    self.names.insert(name);
                }
            }
        }
        syn::visit::visit_expr(self, e);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        use quote::ToTokens;
        for tt in mac.tokens.to_token_stream() {
            if let proc_macro2::TokenTree::Ident(id) = tt {
                self.macro_hits.insert(id.to_string());
            }
        }
        syn::visit::visit_macro(self, mac);
    }

    fn visit_item(&mut self, i: &'ast syn::Item) {
        let name = match i {
            syn::Item::Fn(x) => Some(x.sig.ident.to_string()),
            syn::Item::Struct(x) => Some(x.ident.to_string()),
            syn::Item::Enum(x) => Some(x.ident.to_string()),
            syn::Item::Mod(x) => Some(x.ident.to_string()),
            syn::Item::Const(x) => Some(x.ident.to_string()),
            syn::Item::Static(x) => Some(x.ident.to_string()),
            syn::Item::Trait(x) => Some(x.ident.to_string()),
            syn::Item::Type(x) => Some(x.ident.to_string()),
            syn::Item::Union(x) => Some(x.ident.to_string()),
            _ => None,
        };
        if let Some(n) = name {
            self.names.insert(n);
        }
        syn::visit::visit_item(self, i);
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn used_binding_renames_everywhere() {
        // The P51a production specimen shape (clone_on_copy.rs, cycle 71):
        // `_mod_name` bound and used → `mod_name`; the UNUSED `_root`
        // sibling keeps its underscore.
        let input = "fn f(segs: &[String]) -> bool { let [_root, _mod_name, c, m] = segs else { return false }; _mod_name == \"clone\" && c == m }";
        let out = propose_fix("used_underscore_binding", input).expect("matched");
        assert!(
            out.contains("mod_name == \"clone\""),
            "the use site renames: {out}"
        );
        assert!(
            out.contains("let [_root, mod_name, c, m]"),
            "the binding renames, the unused _root does not: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn unused_binding_stays() {
        let input = "fn f() -> u8 { let _dead = 3; 4 }";
        let out = propose_fix("used_underscore_binding", input);
        assert!(out.is_none(), "an unused _x never renames: {out:?}");
    }

    #[test]
    fn cross_fn_unused_binding_stays() {
        // fn A uses its `_x`; fn B merely binds `_x` — B's binding keeps
        // the underscore (renaming it would birth an unused-var warning).
        let input = "fn a(_x: u8) -> u8 { _x + 1 } fn b(_x: u8) -> u8 { 2 }";
        let out = propose_fix("used_underscore_binding", input).expect("fn a renames");
        assert!(
            out.contains("fn a(x: u8) -> u8 { x + 1 }"),
            "fn a renames its used binding: {out}"
        );
        assert!(
            out.contains("fn b(_x: u8) -> u8 { 2 }"),
            "fn b's unused binding keeps the underscore: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn module_item_use_refuses() {
        // The binding is a module const — upstream's items shape, not ours.
        let input = "const _c: u8 = 1; fn f() -> u8 { _c + 1 }";
        let out = propose_fix("used_underscore_binding", input);
        assert!(
            out.is_none(),
            "uses without an in-fn binding refuse: {out:?}"
        );
    }

    #[test]
    fn target_name_collision_refuses() {
        let input = "fn f(_x: u8, x: u8) -> u8 { x + _x }";
        let out = propose_fix("used_underscore_binding", input);
        assert!(out.is_none(), "a same-file x refuses the rename: {out:?}");
    }

    #[test]
    fn path_name_collision_refuses() {
        // `cnt` exists as an item AND a path use — renaming `_cnt` would
        // shadow-capture the call.
        let input = "fn f(_cnt: u8) -> u8 { cnt() + _cnt } fn cnt() -> u8 { 1 }";
        let out = propose_fix("used_underscore_binding", input);
        assert!(out.is_none(), "a path-ident target name refuses: {out:?}");
    }

    #[test]
    fn fn_param_renames() {
        let input = "fn g(_cnt: u8) -> u8 { _cnt + 1 }";
        let out = propose_fix("used_underscore_binding", input).expect("matched");
        assert!(out.contains("fn g(cnt: u8) -> u8 { cnt + 1 }"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn dunder_refuses() {
        let input = "fn f() -> u8 { let __dunder = 1; __dunder + 1 }";
        let out = propose_fix("used_underscore_binding", input);
        assert!(out.is_none(), "double-underscore refuses: {out:?}");
    }

    #[test]
    fn macro_internal_use_refuses() {
        let input = "fn f(_x: u8) -> u8 { println!(\"{}\", _x); _x + 1 }";
        let out = propose_fix("used_underscore_binding", input);
        assert!(
            out.is_none(),
            "a use inside macro tokens refuses (E0425 strand): {out:?}"
        );
    }

    #[test]
    fn shorthand_pattern_refuses() {
        let input = "fn f(s: S) -> u8 { let S { _v } = s; _v + 1 }";
        let out = propose_fix("used_underscore_binding", input);
        assert!(out.is_none(), "struct-shorthand binding refuses: {out:?}");
    }

    #[test]
    fn same_fn_shadowing_all_rename() {
        // Shadowing topology is preserved when every occurrence renames.
        let input = "fn f(_v: u8) -> u8 { let _v = _v + 1; _v * 2 }";
        let out = propose_fix("used_underscore_binding", input).expect("matched");
        assert!(!out.contains("_v"), "every occurrence renames: {out}");
        syn::parse_file(&out).expect("re-parses");
    }
}

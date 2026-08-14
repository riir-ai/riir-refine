//! `vec-with-capacity` — `Vec::new()`/`vec![]` + push loop → `Vec::with_capacity(n)`.
//!
//! Pattern: a `let mut v = Vec::new();` (or `vec![]`) binding followed, in
//! the same block, by a `for` loop that pushes to `v`. The capacity hint is
//! inferred from the loop header — only forms with an unambiguous capacity
//! source rewrite (an incorrect `.len()` on a type that has none would
//! produce code that doesn't compile):
//!
//! | Loop header | Capacity |
//! |---|---|
//! | `for x in 0..n` | `n` |
//! | `for x in 0..=n` | `n + 1` |
//! | `for x in &items` | `items.len()` |
//! | `for x in items.iter()` / `.iter_mut()` | `items.len()` |
//!
//! `while` loops + unknown iterator expressions are skipped (no capacity
//! source). A shadowing re-`let` of `v` between binding and loop stops the
//! scan — the pushes may belong to the shadow.
//!
//! Span-preserving: the edit replaces exactly the `Vec::new()` init text;
//! the capacity fragment is the ORIGINAL source text of the inferred node.

use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprLit, File, Lit, Local, Pat, RangeLimits, Stmt};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    super::for_each_fn(file, |_sig, block| {
        collect_block_edits(source, block, &mut edits);
    });
    edits
}

fn collect_block_edits(source: &str, block: &syn::Block, edits: &mut Vec<SourceEdit>) {
    let stmts = &block.stmts;
    for (i, stmt) in stmts.iter().enumerate() {
        let Stmt::Local(local) = stmt else { continue };
        let Some(ident) = fresh_vec_binding(local) else { continue };
        for later in stmts.iter().skip(i + 1) {
            // Stop at a shadowing re-bind of the same ident: pushes after it
            // may belong to the shadow, not the binding we'd rewrite.
            if stmt_binds_ident(later, ident) {
                break;
            }
            if let Some(cap) = loop_capacity(later, ident) {
                if let Some(replacement) = cap_replacement(source, &cap) {
                    if let Some(init) = &local.init {
                        edits.push(edit(&init.expr, replacement));
                    }
                }
                break;
            }
        }
    }
}

/// The capacity, as a node whose ORIGINAL source text becomes the
/// `with_capacity` argument (formatting-preserving).
enum Cap {
    /// Use this node's source text verbatim (`0..n` → `n`).
    Node(syn::Expr),
    /// `<node source> + 1` (`0..=n` → `n + 1`).
    NodePlusOne(syn::Expr),
    /// `<node source>.len()` (`&items` / `items.iter()`).
    LenOf(syn::Expr),
}

/// The binding is `let mut <ident> = Vec::new();` / `= vec![];` (no `else`).
fn fresh_vec_binding(local: &Local) -> Option<&syn::Ident> {
    let Pat::Ident(pi) = &local.pat else { return None };
    if pi.by_ref.is_some() || pi.subpat.is_some() || pi.mutability.is_none() {
        return None;
    }
    let init = local.init.as_ref()?;
    if init.diverge.is_some() {
        return None;
    }
    match &*init.expr {
        Expr::Call(call) => {
            let Expr::Path(p) = &*call.func else { return None };
            let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
            if segs == ["Vec", "new"] && call.args.is_empty() {
                Some(&pi.ident)
            } else {
                None
            }
        }
        Expr::Macro(mac) if mac.mac.path.is_ident("vec") && mac.mac.tokens.is_empty() => {
            Some(&pi.ident)
        }
        _ => None,
    }
}

/// Does this stmt re-bind `ident` (any mutability)?
fn stmt_binds_ident(stmt: &Stmt, ident: &syn::Ident) -> bool {
    match stmt {
        Stmt::Local(local) => matches!(&local.pat, Pat::Ident(pi) if pi.ident == *ident),
        _ => false,
    }
}

/// If `stmt` is a `for` loop that (a) pushes to `ident` somewhere in its
/// body and (b) has an unambiguous capacity source, return that capacity.
fn loop_capacity(stmt: &Stmt, ident: &syn::Ident) -> Option<Cap> {
    let Stmt::Expr(Expr::ForLoop(fl), _) = stmt else { return None };
    let mut scan = PushScan { ident, found: false };
    scan.visit_block(&fl.body);
    if !scan.found {
        return None;
    }
    capacity_from_iter(&fl.expr)
}

/// Infer the capacity from the for-loop's iterable.
fn capacity_from_iter(expr: &Expr) -> Option<Cap> {
    match expr {
        // `0..n` → `n`; `0..=n` → `n + 1`. Start must be a literal 0.
        Expr::Range(r) => {
            let starts_zero = matches!(
                r.start.as_deref(),
                Some(Expr::Lit(ExprLit { lit: Lit::Int(li), .. }))
                    if li.base10_parse::<u64>().is_ok_and(|v| v == 0)
            );
            if !starts_zero {
                return None;
            }
            let end = r.end.as_deref()?.clone();
            match r.limits {
                RangeLimits::HalfOpen(_) => Some(Cap::Node(end)),
                RangeLimits::Closed(_) => Some(Cap::NodePlusOne(end)),
            }
        }
        // `&items` → `items.len()` (plain path receivers only — field
        // accesses like `&self.items` parse as Expr::Field, skipped).
        Expr::Reference(er) => match &*er.expr {
            Expr::Path(_) => Some(Cap::LenOf((*er.expr).clone())),
            _ => None,
        },
        // `items.iter()` / `items.iter_mut()` → `items.len()`.
        Expr::MethodCall(mc)
            if (mc.method == "iter" || mc.method == "iter_mut") && mc.args.is_empty() =>
        {
            match &*mc.receiver {
                Expr::Path(_) => Some(Cap::LenOf((*mc.receiver).clone())),
                Expr::Reference(er) => match &*er.expr {
                    Expr::Path(_) => Some(Cap::LenOf((*er.expr).clone())),
                    _ => None,
                },
                _ => None,
            }
        }
        _ => None,
    }
}

/// Build `Vec::with_capacity(<cap text>)` from the ORIGINAL source text of
/// the capacity node.
fn cap_replacement(source: &str, cap: &Cap) -> Option<String> {
    match cap {
        Cap::Node(node) => Some(format!("Vec::with_capacity({})", src(source, node)?)),
        Cap::NodePlusOne(node) => Some(format!("Vec::with_capacity({} + 1)", src(source, node)?)),
        Cap::LenOf(node) => Some(format!("Vec::with_capacity({}.len())", src(source, node)?)),
    }
}

/// Immutable scan: is there a `<ident>.push(_)` method call anywhere in the
/// loop body (including nested ifs + closures)?
struct PushScan<'a> {
    ident: &'a syn::Ident,
    found: bool,
}

impl<'ast> Visit<'ast> for PushScan<'_> {
    fn visit_expr_method_call(&mut self, e: &'ast syn::ExprMethodCall) {
        if self.found {
            return;
        }
        if e.method == "push"
            && e.args.len() == 1
            && matches!(&*e.receiver, Expr::Path(p) if p.path.is_ident(self.ident))
        {
            self.found = true;
            return;
        }
        syn::visit::visit_expr_method_call(self, e);
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    fn squash(s: &str) -> String {
        s.split_whitespace().collect::<String>()
    }

    #[test]
    fn range_loop_gets_capacity() {
        let out = propose_fix(
            "vec-with-capacity",
            "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             for i in 0..n { v.push(i as u32); }\n    v\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains("Vec::with_capacity(n)"), "{out}");
    }

    #[test]
    fn inclusive_range_gets_plus_one() {
        let out = propose_fix(
            "vec-with-capacity",
            "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             for i in 0..=n { v.push(i as u32); }\n    v\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains(&squash("Vec::with_capacity(n + 1)")), "{out}");
    }

    #[test]
    fn ref_iter_gets_len() {
        let out = propose_fix(
            "vec-with-capacity",
            "fn build(items: &Vec<u32>) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             for x in &items { v.push(*x); }\n    v\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains("Vec::with_capacity(items.len())"), "{out}");
    }

    #[test]
    fn iter_method_gets_len() {
        let out = propose_fix(
            "vec-with-capacity",
            "fn build(items: &[u32]) -> Vec<u32> {\n    let mut v = vec![];\n    \
             for x in items.iter() { v.push(*x); }\n    v\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains("Vec::with_capacity(items.len())"), "{out}");
    }

    #[test]
    fn push_in_nested_if_counts() {
        let out = propose_fix(
            "vec-with-capacity",
            "fn build(items: &[u32]) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             for x in items.iter() { if *x > 3 { v.push(*x); } }\n    v\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains("with_capacity"), "{out}");
    }

    #[test]
    fn impl_method_rewrites() {
        let out = propose_fix(
            "vec-with-capacity",
            "struct S;\nimpl S {\n    fn build(&self, n: usize) -> Vec<u8> {\n        \
             let mut v = Vec::new();\n        for i in 0..n { v.push(i as u8); }\n        v\n    }\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains("Vec::with_capacity(n)"), "{out}");
    }

    #[test]
    fn complex_capacity_expression_preserved() {
        // The capacity node's ORIGINAL text (a call expr) is reused verbatim.
        let out = propose_fix(
            "vec-with-capacity",
            "fn build(cfg: &Cfg) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
             for i in 0..cfg.count() { v.push(i as u32); }\n    v\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains("Vec::with_capacity(cfg.count())"), "{out}");
    }

    #[test]
    fn no_loop_leaves_unchanged() {
        assert!(
            propose_fix(
                "vec-with-capacity",
                "fn f() { let mut v = Vec::new(); v.push(1); }"
            )
            .is_none()
        );
    }

    #[test]
    fn while_loop_skipped() {
        assert!(
            propose_fix(
                "vec-with-capacity",
                "fn f(mut it: std::vec::IntoIter<u32>) -> Vec<u32> {\n    let mut v = \
                 Vec::new();\n    while let Some(x) = it.next() { v.push(x); }\n    v\n}"
            )
            .is_none(),
            "while loops have no capacity source"
        );
    }

    #[test]
    fn shadowing_stops_scan() {
        assert!(propose_fix(
            "vec-with-capacity",
            "fn f(items: &[u32]) -> Vec<u32> {\n    let mut v = Vec::new();\n    let mut v = \
             Vec::new();\n    for x in items { v.push(*x); }\n    v\n}"
        )
        .is_none());
    }

    #[test]
    fn already_with_capacity_unchanged() {
        assert!(
            propose_fix(
                "vec-with-capacity",
                "fn f(n: usize) -> Vec<u32> { let mut v = Vec::with_capacity(n); v }"
            )
            .is_none()
        );
    }

    #[test]
    fn nonzero_range_start_skipped() {
        assert!(propose_fix(
            "vec-with-capacity",
            "fn f(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    for i in 1..n { \
             v.push(i as u32); }\n    v\n}"
        )
        .is_none());
    }

    #[test]
    fn bare_path_iterable_unchanged() {
        // `for x in items` (bare path): the binding could be an iterator
        // without `.len()` — emitting `items.len()` might not compile, so the
        // safe design skips it. Use `&items` / `.iter()` / a range instead.
        assert!(
            propose_fix(
                "vec-with-capacity",
                "fn build(items: &[u32]) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
                 for x in items { v.push(*x); }\n    v\n}"
            )
            .is_none()
        );
    }
}

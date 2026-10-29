//! The shared wire-taint walker (Batch 165, Plan 156 T2) — the byte-origin
//! taint substrate BOTH `InputValidation` wire detectors consume.
//!
//! Extracted from `length_prefix_alloc` so the
//! `unchecked-wire-arithmetic` sink (Batch 165) reuses the shipped walk
//! instead of duplicating it (the Batch-131 machinery, unchanged): a length
//! parsed inside a fn (`u32::from_be_bytes(..)`-class byte-swap, or a
//! `.parse()` of a header field) is TRACKED through transparent wrappers
//! (casts, `?`, unwrap family, `Some(..)`, arithmetic), a comparison/
//! match/assert VALIDATES it (and its ancestors), and the in-order walk
//! hands every expression to the per-rule SINK — the alloc sink checks
//! `vec![..; tracked]`, the arithmetic sink checks bare integer operators
//! on a tracked operand.
//!
//! The walker is SINK-AGNOSTIC by construction: it never descends into
//! macro tokens (the `vec!` early-return is each sink's business — the
//! alloc sink owns it; the arithmetic sink simply never sees inside
//! macros, the documented inline-repeat recall limit).

use proc_macro2::{Delimiter, Group};
use quote::ToTokens;
use syn::visit::Visit;
use syn::{BinOp, Expr, File, Item, Stmt};

/// One rule's sink: called for every expression IN SOURCE ORDER, after the
/// validation clearing pass has run on the same expression (so a tracked
/// ident compared anywhere in the expression is already marked validated
/// when the sink sees it).
pub(crate) trait WireSink {
    fn check(&mut self, source: &str, expr: &Expr, state: &FnState);
}

/// Walk a whole file (top-level fns, impl methods, non-test modules — the
/// `#[cfg(test)]` comment guard applies at every nesting depth).
pub(crate) fn walk_file(source: &str, file: &File, sink: &mut impl WireSink) {
    for item in &file.items {
        collect_in_item(source, item, sink);
    }
}

fn collect_in_item(source: &str, item: &Item, sink: &mut impl WireSink) {
    match item {
        Item::Fn(f) => walk_fn(source, &f.block, sink),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    walk_fn(source, &m.block, sink);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production wire paths.
            if is_test_mod(m) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, sink);
                }
            }
        }
        _ => {}
    }
}

pub(crate) fn is_test_mod(m: &syn::ItemMod) -> bool {
    m.attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
}

/// The per-fn tracking state.
#[derive(Default)]
pub(crate) struct FnState {
    /// ident -> its tracking parents (sources it was derived from).
    pub(crate) tracked: std::collections::HashMap<String, Vec<String>>,
    /// idents that saw a bound comparison/match/assert before a sink site.
    pub(crate) validated: std::collections::HashSet<String>,
}

impl FnState {
    pub(crate) fn track(&mut self, ident: &str, parents: Vec<String>) {
        self.tracked.insert(ident.to_string(), parents);
    }

    pub(crate) fn is_tracked(&self, ident: &str) -> bool {
        self.tracked.contains_key(ident)
    }

    /// Validating `ident` also validates every tracked ancestor
    /// (`n = len * 4` checked bounds `len`).
    pub(crate) fn validate(&mut self, ident: &str) {
        let mut queue = vec![ident.to_string()];
        while let Some(id) = queue.pop() {
            if self.validated.insert(id.clone()) {
                if let Some(parents) = self.tracked.get(&id) {
                    queue.extend(parents.iter().cloned());
                }
            }
        }
    }
}

pub(crate) fn walk_fn(source: &str, block: &syn::Block, sink: &mut impl WireSink) {
    let mut state = FnState::default();
    walk_stmts(source, &block.stmts, &mut state, sink);
}

/// Walk statements IN ORDER: bindings track, validations suppress, sinks
/// fire — the same-name-later ordering is what makes the gates honest.
fn walk_stmts(source: &str, stmts: &[Stmt], state: &mut FnState, sink: &mut impl WireSink) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    // `let len = <parse-chain>` → track the bound idents.
                    for pat_ident in local_idents(&l.pat) {
                        if let Some(parents) = harvest_chain(&init.expr, state) {
                            state.track(&pat_ident, parents);
                        }
                    }
                    check_validation(&init.expr, state);
                    walk_expr(source, &init.expr, state, sink);
                }
            }
            Stmt::Item(item) => collect_in_item(source, item, sink),
            Stmt::Expr(e, _) => {
                // `content_length = Some(parse(..))` — the header-variable
                // assignment shape tracks the assigned ident too.
                if let Expr::Assign(a) = e {
                    if let Some(parents) = harvest_chain(&a.right, state) {
                        if let Some(name) = path_ident(&a.left) {
                            state.track(&name, parents);
                        }
                    }
                    check_validation(&a.right, state);
                    walk_expr(source, &a.right, state, sink);
                } else {
                    check_validation(e, state);
                    walk_expr(source, e, state, sink);
                }
            }
            Stmt::Macro(_) => {}
        }
    }
}

/// The `let`-pattern's plain idents (bare + tuple patterns; wildcards and
/// struct patterns are skipped — conservative).
fn local_idents(pat: &syn::Pat) -> Vec<String> {
    let mut out = Vec::new();
    walk_pat(pat, &mut out);
    out
}

fn walk_pat(pat: &syn::Pat, out: &mut Vec<String>) {
    match pat {
        syn::Pat::Ident(p) => out.push(p.ident.to_string()),
        syn::Pat::Tuple(t) => {
            for p in &t.elems {
                walk_pat(p, out);
            }
        }
        syn::Pat::Type(p) => walk_pat(&p.pat, out),
        syn::Pat::Paren(p) => walk_pat(&p.pat, out),
        syn::Pat::Reference(r) => walk_pat(&r.pat, out),
        _ => {}
    }
}

/// A bare single-segment path ident, if the expression is one.
pub(crate) fn path_ident(e: &Expr) -> Option<String> {
    if let Expr::Path(p) = e {
        if p.qself.is_none() && p.path.segments.len() == 1 {
            return Some(p.path.segments[0].ident.to_string());
        }
    }
    None
}

/// Is this expr a length-parse site (`from_be_bytes(..)` assoc call or any
/// `.parse(..)` method call)?
pub(crate) fn is_parse_site(e: &Expr) -> bool {
    match e {
        Expr::Call(c) => match c.func.as_ref() {
            Expr::Path(p) => p.path.segments.last().is_some_and(|s| {
                matches!(
                    s.ident.to_string().as_str(),
                    "from_be_bytes" | "from_le_bytes" | "from_ne_bytes"
                )
            }),
            _ => false,
        },
        Expr::MethodCall(m) => m.method == "parse",
        _ => false,
    }
}

/// If `expr` transparently derives from a tracked ident (or IS a parse
/// site), return the parent idents for the newly tracked binding.
/// Transparent wrappers: casts, `?`, `ok_or`/`ok_or_else`/`unwrap`/
/// `unwrap_or`/`unwrap_or_else`/`expect`/`map_err`, `Some(..)`, and
/// arithmetic on a tracked magnitude (a peer-controlled magnitude squared
/// is still one).
pub(crate) fn harvest_chain(expr: &Expr, state: &FnState) -> Option<Vec<String>> {
    // A fresh parse site binds with no parents.
    if is_parse_site(expr) {
        return Some(Vec::new());
    }
    let from = |e: &Expr| -> Option<Vec<String>> {
        let id = path_ident(e)?;
        state.is_tracked(&id).then(|| vec![id])
    };
    match expr {
        Expr::Cast(c) => harvest_chain(&c.expr, state),
        Expr::Try(t) => harvest_chain(&t.expr, state),
        Expr::Paren(p) => harvest_chain(&p.expr, state),
        Expr::Call(c) => {
            // `Some(<tracked>)`
            if matches!(c.func.as_ref(), Expr::Path(p) if p.path.segments.len() == 1 && p.path.segments[0].ident == "Some")
            {
                if let Some(first) = c.args.first() {
                    return harvest_chain(first, state);
                }
            }
            None
        }
        Expr::MethodCall(m) => {
            if matches!(
                m.method.to_string().as_str(),
                "ok_or"
                    | "ok_or_else"
                    | "unwrap"
                    | "unwrap_or"
                    | "unwrap_or_else"
                    | "expect"
                    | "map_err"
            ) {
                return harvest_chain(&m.receiver, state);
            }
            None
        }
        Expr::Binary(b) => {
            if matches!(
                b.op,
                BinOp::Add(_)
                    | BinOp::Mul(_)
                    | BinOp::Shl(_)
                    | BinOp::Sub(_)
                    | BinOp::Div(_)
                    | BinOp::Shr(_)
            ) {
                let mut parents = harvest_chain(&b.left, state).unwrap_or_default();
                parents.extend(harvest_chain(&b.right, state).unwrap_or_default());
                if parents.is_empty() {
                    None
                } else {
                    Some(parents)
                }
            } else {
                None
            }
        }
        _ => from(expr),
    }
}

/// Mark every tracked ident mentioned in a comparison / match scrutinee /
/// assert as validated (the bounded twin's gate).
pub(crate) fn check_validation(expr: &Expr, state: &mut FnState) {
    struct V<'a>(&'a mut FnState);
    impl Visit<'_> for V<'_> {
        fn visit_expr(&mut self, e: &Expr) {
            if let Expr::Binary(b) = e {
                if matches!(
                    b.op,
                    BinOp::Lt(_)
                        | BinOp::Gt(_)
                        | BinOp::Le(_)
                        | BinOp::Ge(_)
                        | BinOp::Eq(_)
                        | BinOp::Ne(_)
                ) {
                    let mut found = Vec::new();
                    collect_tracked(&b.left, &self.0.tracked, &mut found);
                    collect_tracked(&b.right, &self.0.tracked, &mut found);
                    for id in found {
                        self.0.validate(&id);
                    }
                }
            }
            if let Expr::Match(m) = e {
                let mut found = Vec::new();
                collect_tracked(&m.expr, &self.0.tracked, &mut found);
                for id in found {
                    self.0.validate(&id);
                }
            }
            syn::visit::visit_expr(self, e);
        }
    }
    // `assert!`/`debug_assert!` carry their comparison inside macro tokens.
    if let Expr::Macro(mac) = expr {
        if mac
            .mac
            .path
            .segments
            .last()
            .is_some_and(|s| s.ident.to_string().starts_with("assert"))
        {
            if let Ok(inner) = syn::parse2::<Expr>(mac.mac.tokens.clone()) {
                check_validation(&inner, state);
            }
        }
    }
    let mut v = V(state);
    v.visit_expr(expr);
}

fn collect_tracked(
    e: &Expr,
    tracked: &std::collections::HashMap<String, Vec<String>>,
    out: &mut Vec<String>,
) {
    struct W<'a> {
        tracked: &'a std::collections::HashMap<String, Vec<String>>,
        out: &'a mut Vec<String>,
    }
    impl Visit<'_> for W<'_> {
        fn visit_expr(&mut self, e: &Expr) {
            if let Some(id) = path_ident(e) {
                if self.tracked.contains_key(&id) && !self.out.contains(&id) {
                    self.out.push(id);
                }
            }
            syn::visit::visit_expr(self, e);
        }
    }

    if let Some(id) = path_ident(e) {
        if tracked.contains_key(&id) {
            out.push(id);
        }
        return;
    }
    let mut w = W { tracked, out };
    w.visit_expr(e);
}

/// The `vec![init; count]` macro's parsed repeat, if the expression IS a
/// vec! repeat macro (the alloc sink's shape).
pub(crate) fn vec_repeat_len(expr: &Expr) -> Option<Expr> {
    let Expr::Macro(mac) = expr else {
        return None;
    };
    let is_vec = mac
        .mac
        .path
        .segments
        .last()
        .is_some_and(|s| s.ident == "vec");
    if !is_vec {
        return None;
    }
    // The macro tokens carry `[init; count]` WITHOUT the delimiters —
    // re-wrap in brackets so the token stream parses as the array-repeat
    // expression it expands to.
    let wrapped = Group::new(Delimiter::Bracket, mac.mac.tokens.clone()).to_token_stream();
    match syn::parse2::<Expr>(wrapped) {
        Ok(Expr::Repeat(rep)) => Some(*rep.len),
        _ => None,
    }
}

/// In-order expr walk: validations first (a comparison anywhere in this
/// expr suppresses everything below it), then the sink, then recursion.
/// Macro tokens are never descended into (the sinks' shared recall limit).
pub(crate) fn walk_expr(source: &str, expr: &Expr, state: &mut FnState, sink: &mut impl WireSink) {
    check_validation(expr, state);
    sink.check(source, expr, state);
    // Recurse in syntactic order.
    match expr {
        Expr::If(i) => {
            walk_expr(source, &i.cond, state, sink);
            walk_stmts(source, &i.then_branch.stmts, state, sink);
            if let Some((_, else_e)) = &i.else_branch {
                walk_expr(source, else_e, state, sink);
            }
        }
        Expr::Match(m) => {
            walk_expr(source, &m.expr, state, sink);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    walk_expr(source, guard, state, sink);
                }
                walk_expr(source, &arm.body, state, sink);
            }
        }
        Expr::Block(b) => walk_stmts(source, &b.block.stmts, state, sink),
        Expr::Unsafe(u) => walk_stmts(source, &u.block.stmts, state, sink),
        Expr::Loop(l) => walk_stmts(source, &l.body.stmts, state, sink),
        Expr::While(w) => {
            walk_expr(source, &w.cond, state, sink);
            walk_stmts(source, &w.body.stmts, state, sink);
        }
        Expr::ForLoop(f) => {
            walk_expr(source, &f.expr, state, sink);
            walk_stmts(source, &f.body.stmts, state, sink);
        }
        Expr::MethodCall(m) => {
            walk_expr(source, &m.receiver, state, sink);
            for a in &m.args {
                walk_expr(source, a, state, sink);
            }
        }
        Expr::Call(c) => {
            walk_expr(source, &c.func, state, sink);
            for a in &c.args {
                walk_expr(source, a, state, sink);
            }
        }
        Expr::Assign(a) => {
            walk_expr(source, &a.right, state, sink);
        }
        Expr::Binary(b) => {
            walk_expr(source, &b.left, state, sink);
            walk_expr(source, &b.right, state, sink);
        }
        Expr::Closure(c) => walk_expr(source, &c.body, state, sink),
        _ => {}
    }
}

/// Byte range of an expression, bounded to the source (span integrity —
/// both sinks slice the source with it).
pub(crate) fn bounded_span(expr: &Expr, source: &str) -> Option<std::ops::Range<usize>> {
    let span = syn::spanned::Spanned::span(expr).byte_range();
    (span.end <= source.len()).then_some(span)
}

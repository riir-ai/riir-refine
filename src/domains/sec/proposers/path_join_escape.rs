//! `untrusted-path-join-escape` — SUGGEST-ONLY detector
//! (Batch 258, the cloudflare/security-audit-skill distill; Research 292 /
//! Plan 262).
//!
//! A caller-supplied name joined onto a trusted base and sunk into the
//! filesystem WITHOUT a component check — `fs::read_to_string(base.join(name))`
//! with `name: &str` a fn parameter — lets `..`, absolute components, or
//! Windows prefixes ESCAPE the base directory: the path-traversal class
//! (read any file the service's user can read, write the export outside
//! the export tree). The bounded repair is the `components()` walk refusing
//! everything but `Component::Normal(_)` — the in-house riir-deployer
//! `48d63f7` files-row fix shape, cited in-rule.
//!
//! ## Why suggest-only (never a fix)
//!
//! The refusal path's error type is the fn's choice (the
//! `ffi-export-panicking-body` class): inventing one is judgment-bearing.
//! This module ships a parse-only DETECTION walk only: NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm — the suggestion surfaces through
//! retrieval (`--suggest` / the corpus entry) and review.
//!
//! ## The bounded form
//!
//! ```text
//! let joined = base.join(name);
//! if joined.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
//!     return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "path escapes base"));
//! }
//! std::fs::read_to_string(joined)
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Guard vocabulary suppresses fn-wide** (one hardened fn hides a
//!   sibling — the `secret-file-create` conservative rule): `.components()`,
//!   the `Component::ParentDir` / `RootDir` / `CurDir` / `Prefix` spellings,
//!   or `canonicalize` anywhere in the fn → silent.
//! - **Lexical `starts_with` / `strip_prefix` do NOT suppress** (by design,
//!   the rule's subtle half): a joined `..` survives both lexical checks —
//!   `base.join("../../etc")` still `starts_with(base)` — so firing there
//!   is a TRUE finding, and the honest limits say so.
//! - **Literal / const join args never fire** (trusted by construction):
//!   `base.join("cfg.toml")` and `base.join(CONFIG_NAME)` are silent.
//! - **Param-origin vocabulary, parse-provable only**: the joined argument
//!   is a bare fn-parameter ident (or a transparent converter on one —
//!   `name.to_string()` / `.as_str()` / `.to_owned()` / `.to_path_buf()`).
//!   Field origins (`row.file_name` — the `read_dir` iteration idiom) are
//!   an honest NO: the dominant benign shape stays silent.
//! - **A filesystem sink is required** (`fs::{read, read_to_string, write,
//!   create_dir_all, remove_*, rename, copy, read_dir, metadata, ...}`,
//!   `File::{open, create}`): a join that only feeds `.display()` or a
//!   response string is out of scope.
//! - **`canonicalize` is a guard ingredient, not a sink** — excluded from
//!   the sink set (and its presence suppresses the fn).
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped;
//!   macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - The shape proves a MISSING GUARD, not an attacker — a helper joining
//!   two internal parameters fires too, and review decides. Suggest-only
//!   is the blast-radius control.
//! - Cross-fn flows (join in a helper, sink in the caller) are not
//!   connected parse-only.
//! - `OsStr`-from-`bytes` / `CString`-style constructions and any join
//!   inside a macro are unreached.
//! - Batch 258 / Plan 262 / Research 292 (cloudflare/security-audit-skill
//!   @ `c1c8a8c`, MIT — cited as class provenance; the fixture is an
//!   original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item, Stmt};

/// One detected unguarded join-then-sink flow.
#[derive(Debug, Clone)]
pub struct PathJoinFinding {
    /// Byte range of the `join(..)` call (the untrusted merge site).
    pub join_span: Range<usize>,
    /// The join expression's source text.
    pub join_text: String,
    /// The joined parameter ident.
    pub joined_ident: String,
    /// The filesystem sink the joined path reaches.
    pub sink_text: String,
    /// The advice line (the `components()` refusal + the failure class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "refuse every non-Normal component before the sink: \
`if joined.components().any(|c| !matches!(c, std::path::Component::Normal(_))) { \
return Err(..) }` — a joined `..`, absolute path, or Windows prefix escapes the \
base directory (path traversal; Batch 258, suggest-only)";

/// The fn-wide guard spellings (case-sensitive — the standard `Component`
/// forms; `strip_prefix`/`starts_with` deliberately NOT here, see the
/// module doc's weak-guard clause).
const GUARD_MARKERS: &[&str] = &[
    ".components()",
    "::ParentDir",
    "::RootDir",
    "::CurDir",
    "::Prefix",
    "canonicalize",
];

/// Transparent converters that keep the param origin of the joined arg.
const PARAM_CONVERTERS: &[&str] = &["to_string", "as_str", "to_owned", "to_path_buf"];

/// The filesystem sinks (`last-two-segments` pairs — the fully-qualified
/// `std::fs::read` form is three).
const SINKS: &[(&str, &str)] = &[
    ("fs", "read_to_string"),
    ("fs", "read"),
    ("fs", "write"),
    ("fs", "create_dir"),
    ("fs", "create_dir_all"),
    ("fs", "remove_file"),
    ("fs", "remove_dir"),
    ("fs", "remove_dir_all"),
    ("fs", "rename"),
    ("fs", "copy"),
    ("fs", "read_dir"),
    ("fs", "hard_link"),
    ("fs", "symlink_metadata"),
    ("fs", "metadata"),
    ("fs", "set_permissions"),
    ("File", "open"),
    ("File", "create"),
    ("File", "create_new"),
];

/// Detect unguarded untrusted-join paths reaching filesystem sinks
/// (suggest-only; no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<PathJoinFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<PathJoinFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, f, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_shaped(source, &m.sig, &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production path flows.
            if is_test_mod(m) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, findings);
                }
            }
        }
        _ => {}
    }
}

fn is_test_mod(m: &syn::ItemMod) -> bool {
    m.attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
}

fn scan_fn(source: &str, f: &syn::ItemFn, findings: &mut Vec<PathJoinFinding>) {
    scan_shaped(source, &f.sig, &f.block, findings);
}

fn scan_shaped(
    source: &str,
    sig: &syn::Signature,
    block: &syn::Block,
    findings: &mut Vec<PathJoinFinding>,
) {
    let params: std::collections::HashSet<String> = sig
        .inputs
        .iter()
        .filter_map(|input| match input {
            syn::FnArg::Typed(pt) => {
                let mut out = Vec::new();
                collect_pat_idents(&pt.pat, &mut out);
                out.into_iter().next()
            }
            syn::FnArg::Receiver(_) => None,
        })
        .collect();
    if params.is_empty() {
        return;
    }
    // fn-wide guard gate (token-text scan, space-stripped — token streams
    // render with separating spaces; case-sensitive, the standard
    // `Component` spellings).
    let sig_ts = quote::ToTokens::to_token_stream(sig).to_string();
    let block_ts = quote::ToTokens::to_token_stream(block).to_string();
    let fn_text = format!("{sig_ts}{block_ts}").replace(' ', "");
    if GUARD_MARKERS.iter().any(|m| fn_text.contains(m)) {
        return;
    }

    // Pass 1: fn-wide join sites + the locals bound from one.
    let mut sites: Vec<(Range<usize>, String, String)> = Vec::new(); // (span, param ident, join text)
    collect_join_sites(block, &params, &mut sites);
    let mut join_bound: std::collections::HashMap<String, (Range<usize>, String, String)> =
        std::collections::HashMap::new();
    collect_join_bound_locals(block, &params, &sites, &mut join_bound);
    if sites.is_empty() && join_bound.is_empty() {
        return;
    }

    // Pass 2: sink walk — which joins reach a filesystem sink.
    let mut reported: std::collections::HashSet<Range<usize>> = std::collections::HashSet::new();
    scan_sinks(source, block, &sites, &join_bound, &mut reported, findings);
}

/// Every `.join(<param-origin>)` site in the block, in source order.
fn collect_join_sites(
    block: &syn::Block,
    params: &std::collections::HashSet<String>,
    out: &mut Vec<(Range<usize>, String, String)>,
) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    collect_join_sites_expr(&init.expr, params, out);
                }
            }
            Stmt::Item(item) => collect_join_sites_item(item, params, out),
            Stmt::Expr(e, _) => collect_join_sites_expr(e, params, out),
            Stmt::Macro(_) => {}
        }
    }
}

fn collect_join_sites_item(
    item: &Item,
    params: &std::collections::HashSet<String>,
    out: &mut Vec<(Range<usize>, String, String)>,
) {
    match item {
        Item::Fn(f) => collect_join_sites(&f.block, params, out),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    collect_join_sites(&m.block, params, out);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_join_sites_item(nested, params, out);
                }
            }
        }
        _ => {}
    }
}

fn collect_join_sites_expr(
    e: &Expr,
    params: &std::collections::HashSet<String>,
    out: &mut Vec<(Range<usize>, String, String)>,
) {
    if let Expr::MethodCall(m) = e {
        if m.method == "join" {
            if let Some(id) = param_origin(&m.args, params) {
                let span = e.span().byte_range();
                out.push((span, id, String::new()));
                return; // do not double-report nested joins of the same site
            }
        }
    }
    walk_children(e, &mut |child: &Expr| {
        collect_join_sites_expr(child, params, out);
    });
}

/// Locals whose initializer CONTAINS a join site (the `let joined = ..;`
/// binding form) — the local inherits the join site as its report anchor.
fn collect_join_bound_locals(
    block: &syn::Block,
    params: &std::collections::HashSet<String>,
    sites: &[(Range<usize>, String, String)],
    out: &mut std::collections::HashMap<String, (Range<usize>, String, String)>,
) {
    if sites.is_empty() {
        return;
    }
    for stmt in &block.stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    let mut inits: Vec<(Range<usize>, String, String)> = Vec::new();
                    collect_join_sites_expr(&init.expr, params, &mut inits);
                    if !inits.is_empty() {
                        let site = inits[0].clone();
                        let mut pats = Vec::new();
                        collect_pat_idents(&l.pat, &mut pats);
                        for p in pats {
                            out.entry(p).or_insert(site.clone());
                        }
                    }
                }
            }
            Stmt::Item(item) => {
                collect_join_bound_locals_item(item, params, sites, out);
            }
            Stmt::Expr(e, _) => collect_join_bound_locals_expr(e, params, sites, out),
            Stmt::Macro(_) => {}
        }
    }
}

fn collect_join_bound_locals_item(
    item: &Item,
    params: &std::collections::HashSet<String>,
    sites: &[(Range<usize>, String, String)],
    out: &mut std::collections::HashMap<String, (Range<usize>, String, String)>,
) {
    match item {
        Item::Fn(f) => collect_join_bound_locals(&f.block, params, sites, out),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    collect_join_bound_locals(&m.block, params, sites, out);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_join_bound_locals_item(nested, params, sites, out);
                }
            }
        }
        _ => {}
    }
}

fn collect_join_bound_locals_expr(
    e: &Expr,
    params: &std::collections::HashSet<String>,
    sites: &[(Range<usize>, String, String)],
    out: &mut std::collections::HashMap<String, (Range<usize>, String, String)>,
) {
    match e {
        Expr::Block(b) => collect_join_bound_locals(&b.block, params, sites, out),
        Expr::Unsafe(u) => collect_join_bound_locals(&u.block, params, sites, out),
        Expr::Loop(l) => collect_join_bound_locals(&l.body, params, sites, out),
        Expr::While(w) => {
            collect_join_bound_locals_expr(&w.cond, params, sites, out);
            collect_join_bound_locals(&w.body, params, sites, out);
        }
        Expr::If(i) => {
            collect_join_bound_locals_expr(&i.cond, params, sites, out);
            collect_join_bound_locals(&i.then_branch, params, sites, out);
            if let Some((_, else_e)) = &i.else_branch {
                collect_join_bound_locals_expr(else_e, params, sites, out);
            }
        }
        Expr::Match(m) => {
            for arm in &m.arms {
                collect_join_bound_locals_expr(&arm.body, params, sites, out);
            }
        }
        _ => {}
    }
}

/// Is one of `args` a bare param ident (or a transparent converter on one)?
fn param_origin(
    args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>,
    params: &std::collections::HashSet<String>,
) -> Option<String> {
    args.iter().find_map(|a| {
        let inner = strip_refs(a);
        match inner {
            Expr::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 => {
                let id = p.path.segments[0].ident.to_string();
                params.contains(&id).then_some(id)
            }
            Expr::MethodCall(m) => {
                if PARAM_CONVERTERS.contains(&m.method.to_string().as_str()) {
                    if let Expr::Path(p) = m.receiver.as_ref() {
                        if p.qself.is_none() && p.path.segments.len() == 1 {
                            let id = p.path.segments[0].ident.to_string();
                            return params.contains(&id).then_some(id);
                        }
                    }
                }
                None
            }
            _ => None,
        }
    })
}

fn strip_refs(mut e: &Expr) -> &Expr {
    loop {
        match e {
            Expr::Reference(r) => e = r.expr.as_ref(),
            Expr::Paren(p) => e = p.expr.as_ref(),
            Expr::Try(t) => e = t.expr.as_ref(),
            _ => return e,
        }
    }
}

/// The sink walk: filesystem sinks whose args carry a join site (inline or
/// via a join-bound local). Each join site reports at most once.
fn scan_sinks(
    source: &str,
    block: &syn::Block,
    sites: &[(Range<usize>, String, String)],
    join_bound: &std::collections::HashMap<String, (Range<usize>, String, String)>,
    reported: &mut std::collections::HashSet<Range<usize>>,
    findings: &mut Vec<PathJoinFinding>,
) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_sinks_expr(source, &init.expr, sites, join_bound, reported, findings);
                }
            }
            Stmt::Item(item) => {
                scan_sinks_item(source, item, sites, join_bound, reported, findings);
            }
            Stmt::Expr(e, _) => scan_sinks_expr(source, e, sites, join_bound, reported, findings),
            Stmt::Macro(_) => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_sinks_item(
    source: &str,
    item: &Item,
    sites: &[(Range<usize>, String, String)],
    join_bound: &std::collections::HashMap<String, (Range<usize>, String, String)>,
    reported: &mut std::collections::HashSet<Range<usize>>,
    findings: &mut Vec<PathJoinFinding>,
) {
    match item {
        Item::Fn(f) => scan_sinks(source, &f.block, sites, join_bound, reported, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_sinks(source, &m.block, sites, join_bound, reported, findings);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    scan_sinks_item(source, nested, sites, join_bound, reported, findings);
                }
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_sinks_expr(
    source: &str,
    e: &Expr,
    sites: &[(Range<usize>, String, String)],
    join_bound: &std::collections::HashMap<String, (Range<usize>, String, String)>,
    reported: &mut std::collections::HashSet<Range<usize>>,
    findings: &mut Vec<PathJoinFinding>,
) {
    // The sink site itself.
    if let Expr::Call(c) = e {
        if let Expr::Path(p) = c.func.as_ref() {
            if p.qself.is_none() && p.path.segments.len() >= 2 {
                let pair = (
                    p.path.segments[p.path.segments.len() - 2].ident.to_string(),
                    p.path.segments[p.path.segments.len() - 1].ident.to_string(),
                );
                if SINKS.contains(&(pair.0.as_str(), pair.1.as_str())) {
                    let sink_span = e.span().byte_range();
                    let sink_text = if sink_span.end <= source.len() {
                        source[sink_span].to_string()
                    } else {
                        pair.1.clone()
                    };
                    for a in &c.args {
                        report_joins_in(
                            source, a, sites, join_bound, &pair.1, &sink_text, reported, findings,
                        );
                    }
                }
            }
        }
    }
    walk_children(e, &mut |child| {
        scan_sinks_expr(source, child, sites, join_bound, reported, findings);
    });
}

/// Report every join site (inline or via a join-bound local) inside `e`.
#[allow(clippy::too_many_arguments)]
fn report_joins_in(
    source: &str,
    e: &Expr,
    sites: &[(Range<usize>, String, String)],
    join_bound: &std::collections::HashMap<String, (Range<usize>, String, String)>,
    sink_name: &str,
    sink_text: &str,
    reported: &mut std::collections::HashSet<Range<usize>>,
    findings: &mut Vec<PathJoinFinding>,
) {
    // Inline join site.
    if let Expr::MethodCall(m) = e {
        if m.method == "join" {
            let span = e.span().byte_range();
            if let Some((_, id, _)) = sites.iter().find(|(s, _, _)| *s == span) {
                push(source, span, id, sink_name, sink_text, reported, findings);
            }
            return; // the join args are not separately interesting
        }
    }
    // Join-bound local use.
    if let Expr::Path(p) = strip_refs(e) {
        if p.qself.is_none() && p.path.segments.len() == 1 {
            let id = p.path.segments[0].ident.to_string();
            if let Some((span, jid, _)) = join_bound.get(&id) {
                push(
                    source,
                    span.clone(),
                    jid,
                    sink_name,
                    sink_text,
                    reported,
                    findings,
                );
                return;
            }
        }
    }
    walk_children(e, &mut |child| {
        report_joins_in(
            source, child, sites, join_bound, sink_name, sink_text, reported, findings,
        );
    });
}

#[allow(clippy::too_many_arguments)]
fn push(
    source: &str,
    span: Range<usize>,
    joined_ident: &str,
    sink_name: &str,
    sink_text: &str,
    reported: &mut std::collections::HashSet<Range<usize>>,
    findings: &mut Vec<PathJoinFinding>,
) {
    if span.end > source.len() || !reported.insert(span.clone()) {
        return;
    }
    findings.push(PathJoinFinding {
        join_text: source[span.clone()].to_string(),
        join_span: span,
        joined_ident: joined_ident.to_string(),
        sink_text: format!("{sink_name} — {sink_text}"),
        advice: ADVICE,
    });
}

/// Recursive child walk over the control-flow shapes (macro tokens are
/// never descended into — the house recall limit).
fn walk_children(e: &Expr, f: &mut impl FnMut(&Expr)) {
    match e {
        Expr::If(i) => {
            f(&i.cond);
            walk_stmts_children(&i.then_branch.stmts, &mut *f);
            if let Some((_, else_e)) = &i.else_branch {
                f(else_e);
            }
        }
        Expr::Match(m) => {
            f(&m.expr);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    f(guard);
                }
                f(&arm.body);
            }
        }
        Expr::Block(b) => walk_stmts_children(&b.block.stmts, &mut *f),
        Expr::Unsafe(u) => walk_stmts_children(&u.block.stmts, &mut *f),
        Expr::Loop(l) => walk_stmts_children(&l.body.stmts, &mut *f),
        Expr::While(w) => {
            f(&w.cond);
            walk_stmts_children(&w.body.stmts, &mut *f);
        }
        Expr::ForLoop(fl) => {
            f(&fl.expr);
            walk_stmts_children(&fl.body.stmts, &mut *f);
        }
        Expr::MethodCall(m) => {
            f(&m.receiver);
            for a in &m.args {
                f(a);
            }
        }
        Expr::Call(c) => {
            f(&c.func);
            for a in &c.args {
                f(a);
            }
        }
        Expr::Assign(a) => f(&a.right),
        Expr::Binary(b) => {
            f(&b.left);
            f(&b.right);
        }
        Expr::Return(r) => {
            if let Some(inner) = &r.expr {
                f(inner);
            }
        }
        Expr::Await(a) => f(&a.base),
        Expr::Field(fd) => f(&fd.base),
        Expr::Index(ix) => {
            f(&ix.expr);
            f(&ix.index);
        }
        Expr::Cast(c) => f(&c.expr),
        Expr::Array(a) => {
            for el in &a.elems {
                f(el);
            }
        }
        Expr::Tuple(t) => {
            for el in &t.elems {
                f(el);
            }
        }
        Expr::Let(lt) => f(&lt.expr),
        Expr::Closure(c) => f(&c.body),
        Expr::Try(t) => f(&t.expr),
        Expr::Reference(r) => f(&r.expr),
        Expr::Paren(p) => f(&p.expr),
        _ => {}
    }
}

fn walk_stmts_children(stmts: &[Stmt], f: &mut impl FnMut(&Expr)) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    f(&init.expr);
                }
            }
            Stmt::Item(item) => walk_item_exprs(item, &mut *f),
            Stmt::Expr(e, _) => f(e),
            Stmt::Macro(_) => {}
        }
    }
}

fn walk_item_exprs(item: &Item, f: &mut impl FnMut(&Expr)) {
    match item {
        Item::Fn(fn_item) => walk_stmts_children(&fn_item.block.stmts, &mut *f),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    walk_stmts_children(&m.block.stmts, &mut *f);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    walk_item_exprs(nested, &mut *f);
                }
            }
        }
        _ => {}
    }
}

fn collect_pat_idents(pat: &syn::Pat, out: &mut Vec<String>) {
    match pat {
        syn::Pat::Ident(p) => out.push(p.ident.to_string()),
        syn::Pat::Tuple(t) => {
            for p in &t.elems {
                collect_pat_idents(p, out);
            }
        }
        syn::Pat::Type(p) => collect_pat_idents(&p.pat, out),
        syn::Pat::Paren(p) => collect_pat_idents(&p.pat, out),
        syn::Pat::Reference(r) => collect_pat_idents(&r.pat, out),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::PathJoinFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 258 gate — the corpus `buggy_code` MUST fire.
    #[test]
    fn batch258_unguarded_param_join_fires() {
        let src = "pub fn read_asset(base: &std::path::Path, name: &str) -> std::io::Result<String> {\n    \
             std::fs::read_to_string(base.join(name))\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].joined_ident, "name");
        assert!(
            f[0].sink_text.starts_with("read_to_string"),
            "{:?}",
            f[0].sink_text
        );
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The guarded (corpus fixed) form stays silent — the `components()`
    /// gate suppresses fn-wide.
    #[test]
    fn components_guard_suppresses() {
        let src = "pub fn read_asset(base: &std::path::Path, name: &str) -> std::io::Result<String> {\n    \
             let joined = base.join(name);\n    \
             if joined.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {\n        \
             return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, \"bad path\"));\n    \
             }\n    \
             std::fs::read_to_string(joined)\n}\n";
        assert!(findings(src).is_empty(), "guarded fn must be silent");
    }

    /// Literal + const join args never fire (trusted by construction).
    #[test]
    fn literal_and_const_args_silent() {
        let src = "const CFG: &str = \"cfg.toml\";\n\n\
             pub fn load(base: &std::path::Path) -> std::io::Result<String> {\n    \
             std::fs::read_to_string(base.join(\"app.toml\"))?;\n    \
             std::fs::read_to_string(base.join(CFG))\n}\n";
        assert!(findings(src).is_empty(), "literal/const joins are trusted");
    }

    /// No filesystem sink — out of scope.
    #[test]
    fn join_without_sink_silent() {
        let src = "pub fn display(base: &std::path::Path, name: &str) -> String {\n    \
             base.join(name).display().to_string()\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Lexical `starts_with` does NOT suppress — the `..` escape survives
    /// it (the rule's subtle half, a TRUE finding).
    #[test]
    fn lexical_starts_with_is_not_a_guard() {
        let src = "pub fn read_checked(base: &std::path::Path, name: &str) -> std::io::Result<Vec<u8>> {\n    \
             let joined = base.join(name);\n    \
             if joined.starts_with(base) {\n        \
             return std::fs::read(joined);\n    \
             }\n    \
             Ok(Vec::new())\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "starts_with must not suppress: {f:?}");
    }

    /// Transparent converters keep the param origin.
    #[test]
    fn converter_keeps_param_origin() {
        let src = "pub fn read_named(base: &std::path::Path, name: &str) -> std::io::Result<Vec<u8>> {\n    \
             std::fs::read(base.join(name.to_string()))\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "to_string keeps the origin: {f:?}");
    }

    /// Field origins stay silent — the `read_dir` iteration idiom is the
    /// dominant benign shape (the honest NO).
    #[test]
    fn field_origin_silent() {
        let src = "pub fn copy_all(base: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {\n    \
             for entry in std::fs::read_dir(base)? {\n        \
             let entry = entry?;\n        \
             std::fs::copy(base.join(entry.file_name()), dst.join(entry.file_name()))?;\n    \
             }\n    \
             Ok(())\n}\n";
        assert!(findings(src).is_empty(), "field origins are an honest NO");
    }

    /// Cross-fn flow: join in a helper, sink in the caller — not
    /// connected parse-only.
    #[test]
    fn cross_fn_flow_undetected() {
        let src = "fn join_helper(base: &std::path::Path, name: &str) -> std::path::PathBuf {\n    \
             base.join(name)\n}\n\n\
             pub fn read_via_helper(base: &std::path::Path, name: &str) -> std::io::Result<String> {\n    \
             std::fs::read_to_string(join_helper(base, name))\n}\n";
        assert!(findings(src).is_empty(), "cross-fn joins are not connected");
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn t(base: &std::path::Path, name: &str) {\n        \
             let _ = std::fs::read_to_string(base.join(name));\n    \
             }\n}\n";
        assert!(findings(src).is_empty());
    }
}

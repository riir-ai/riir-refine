//! `unbounded-read-into-vec` — SUGGEST-ONLY detector
//! (Batch 140, the rustsec/advisory-db distill; Research 161 / Plan 120).
//!
//! A read that grows a `Vec` WITHOUT a byte cap from an untrusted reader —
//! `read_to_end(&mut buf)` (or a `loop { read?; buf.extend(..) }` drain)
//! whose reader is a `TcpStream` (a listener's `accept()`/`incoming()`
//! child, a `connect()` dial-out) — lets one peer control the allocation:
//! the buffer grows without limit on peer-controlled bytes (OOM/abort).
//! The cost axis is security, not cycles. Real-world anchor: the h2
//! RUSTSEC-2024-0003 / CVE-2019-9514 resource-exhaustion class.
//!
//! ## Why suggest-only (never a fix)
//!
//! The bounded repair must CHOOSE the limit (bytes? messages? wall time?)
//! and an error path — a protocol decision, never mechanical. This module
//! therefore ships a parse-only DETECTION walk only: NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm — the suggestion surfaces through
//! retrieval (`--suggest` / the corpus entry) and review.
//!
//! ## The bounded form
//!
//! ```text
//! let mut limited = std::io::Read::take(stream, MAX_BYTES);
//! limited.read_to_end(&mut buf)?;          // + Vec::with_capacity hint
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Untrusted-source vocabulary, parse-provable only**: a reader
//!   tracked from `TcpStream::connect(..)`, any `.accept()` /
//!   `.incoming()` / `.bytes_stream()` call, `reqwest::get(..)`, or a
//!   binding whose type annotation ends in `TcpStream`. A bare `Read`
//!   parameter or an unknown reader is an honest NO (recall limit, not a
//!   match) — naming a variable `stream` proves nothing.
//! - **Trusted regime is out**: `File::open`/`File::create`/
//!   `OpenOptions`/`fs::read*` readers never track — reading a local
//!   config file to the end is idiomatic, and the rule would drown in it.
//! - **`.take(` anywhere in the receiver chain suppresses** (the bounded
//!   twin `stream.take(MAX).read_to_end(&mut buf)`); a `take`-produced
//!   binding is a fresh untracked reader, so the UFCS fixed form is
//!   silent by construction too.
//! - **Loop-drain shape needs BOTH halves**: a `read`/`read_exact` on a
//!   tracked reader AND an `extend`/`extend_from_slice` on a local Vec in
//!   the same loop. Caps suppress: a `resize`/`resize_with` on the target
//!   Vec anywhere in the fn (the capacity-capped drain), `buf.len()` in a
//!   comparison inside the loop, or the READ-COUNT binding (the local the
//!   read was assigned to) compared against a NON-ZERO operand. The EOF
//!   idiom `if n == 0 { break }` does NOT suppress — zero is the
//!   end-of-stream sentinel, not a cap.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - A reader passed as a bare `R: Read` generic or a `dyn Read` is
//!   untracked (the type proves nothing about trust).
//! - Cross-fn drains (reader built in a helper, drained in the caller)
//!   are not connected parse-only.
//! - `Read::read_exact` into a PRE-SIZED buffer from a tracked reader
//!   fires in the loop shape only when the buffer ALSO grows via
//!   `extend` — a fixed-frame read loop is bounded by construction.
//! - The vocabulary proves a SOURCE CLASS, not peer control — a
//!   `connect()` to localhost fires too, and review decides.
//!   Suggest-only is the blast-radius control.

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{BinOp, Expr, File, Item, Stmt};

/// One detected unbounded read into a growable Vec.
#[derive(Debug, Clone)]
pub struct UnboundedReadFinding {
    /// Byte range of the read expression.
    pub read_span: Range<usize>,
    /// The read expression's source text.
    pub read_text: String,
    /// The tracked reader ident (or the source call label for direct
    /// `TcpStream::connect(..)?.read_to_end(..)` chains).
    pub reader_ident: String,
    /// The advice line (the bounded form + the failure class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "bound the read: chain `Read::take(reader, MAX_BYTES)` \
(cap bytes, not messages) and size the buffer with `Vec::with_capacity(..)` — \
an unbounded read from a network reader grows the Vec without limit on \
peer-controlled input (RUSTSEC-2024-0003 / CVE-2019-9514 class; \
Batch 140, suggest-only)";

/// Detect unbounded reads into growable Vecs from tracked network readers
/// (suggest-only; no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<UnboundedReadFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<UnboundedReadFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, f, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn_shaped_sig(source, &m.sig, &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production network paths.
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
    m.attrs.iter().any(|a| {
        a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test")
    })
}

/// The fn-wide binding facts (pass 1).
#[derive(Default)]
struct ReadState {
    /// idents bound (transitively) to an untrusted-reader source.
    untrusted: std::collections::HashSet<String>,
    /// local Vec idents (the loop-drain growth targets).
    vecs: std::collections::HashSet<String>,
    /// Vec idents that saw a `resize`/`resize_with` (capacity pre-set).
    resized: std::collections::HashSet<String>,
}

fn scan_fn(source: &str, f: &syn::ItemFn, findings: &mut Vec<UnboundedReadFinding>) {
    scan_fn_shaped_sig(source, &f.sig, &f.block, findings);
}

fn scan_fn_shaped_sig(
    source: &str,
    sig: &syn::Signature,
    block: &syn::Block,
    findings: &mut Vec<UnboundedReadFinding>,
) {
    let mut state = ReadState::default();
    // TcpStream-typed params (incl. `&mut TcpStream`) are the tracked
    // readers — the parse-provable parameter vocabulary.
    for input in &sig.inputs {
        if let syn::FnArg::Typed(pt) = input {
            if last_ty_segment(&pt.ty).is_some_and(|s| s == "TcpStream") {
                if let syn::Pat::Ident(pi) = pt.pat.as_ref() {
                    state.untrusted.insert(pi.ident.to_string());
                }
            }
        }
    }
    collect_bindings(block, &mut state);
    scan_block_sinks(source, block, &state, findings);
}

// ── pass 1: the fn-wide binding facts ─────────────────────────────────────

fn collect_bindings(block: &syn::Block, state: &mut ReadState) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    let pats = local_idents(&l.pat);
                    if let Some(ty_tail) = local_ty_tail(&l.pat) {
                        if ty_tail == "TcpStream" {
                            for p in &pats {
                                state.untrusted.insert(p.clone());
                            }
                        }
                    }
                    if is_source_expr(&init.expr) {
                        for p in &pats {
                            state.untrusted.insert(p.clone());
                        }
                    }
                    note_local_vec(&l.pat, &init.expr, state);
                    note_resize(&init.expr, state);
                }
            }
            Stmt::Item(item) => collect_in_item_bindings(item, state),
            Stmt::Expr(e, _) => {
                note_resize(e, state);
                note_for_loop_sources(e, state);
            }
            Stmt::Macro(_) => {}
        }
    }
}

fn collect_in_item_bindings(item: &Item, state: &mut ReadState) {
    match item {
        Item::Fn(f) => collect_bindings(&f.block, state),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    collect_bindings(&m.block, state);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item_bindings(nested, state);
                }
            }
        }
        _ => {}
    }
}

fn note_for_loop_sources(e: &Expr, state: &mut ReadState) {
    if let Expr::ForLoop(fl) = e {
        // `for stream in listener.incoming() { .. }` — the pattern binds
        // tracked readers when the iterable is a source.
        if is_source_expr(&fl.expr) {
            for p in local_idents(&fl.pat) {
                state.untrusted.insert(p);
            }
        }
        collect_bindings(&fl.body, state);
        return;
    }
    // Recurse into the other control-flow containers so nested bindings
    // are seen fn-wide.
    match e {
        Expr::While(w) => {
            note_resize(&w.cond, state);
            note_for_loop_sources(&w.cond, state);
            collect_bindings(&w.body, state);
        }
        Expr::Loop(l) => collect_bindings(&l.body, state),
        Expr::If(i) => {
            note_resize(&i.cond, state);
            note_for_loop_sources(&i.cond, state);
            collect_bindings(&i.then_branch, state);
            if let Some((_, else_e)) = &i.else_branch {
                note_for_loop_sources(else_e, state);
            }
        }
        Expr::Match(m) => {
            note_resize(&m.expr, state);
            for arm in &m.arms {
                note_for_loop_sources(&arm.body, state);
            }
        }
        Expr::Block(b) => collect_bindings(&b.block, state),
        _ => {}
    }
}

fn note_local_vec(pat: &syn::Pat, init: &Expr, state: &mut ReadState) {
    let is_vec_init = match init {
        Expr::Call(c) => matches!(c.func.as_ref(), Expr::Path(p)
            if p.path.segments.len() == 2
                && p.path.segments[0].ident == "Vec"
                && matches!(p.path.segments[1].ident.to_string().as_str(), "new" | "with_capacity")),
        Expr::MethodCall(m) => {
            matches!(m.method.to_string().as_str(), "default")
                && matches!(m.receiver.as_ref(), Expr::Path(p)
                    if p.path.segments.last().is_some_and(|s| s.ident == "Vec"))
        }
        Expr::Macro(mac) => mac.mac.path.is_ident("vec"),
        _ => false,
    };
    if is_vec_init {
        for p in local_idents(pat) {
            state.vecs.insert(p);
        }
    }
}

fn note_resize(e: &Expr, state: &mut ReadState) {
    if let Expr::MethodCall(m) = e {
        if matches!(m.method.to_string().as_str(), "resize" | "resize_with") {
            if let Some(id) = path_ident(&m.receiver) {
                state.resized.insert(id);
            }
        }
    }
}

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

fn local_ty_tail(pat: &syn::Pat) -> Option<String> {
    if let syn::Pat::Type(p) = pat {
        return last_ty_segment(&p.ty);
    }
    None
}

fn last_ty_segment(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(tp) => tp.path.segments.last().map(|s| s.ident.to_string()),
        syn::Type::Reference(r) => last_ty_segment(&r.elem),
        syn::Type::Paren(p) => last_ty_segment(&p.elem),
        _ => None,
    }
}

fn path_ident(e: &Expr) -> Option<String> {
    if let Expr::Path(p) = e {
        if p.qself.is_none() && p.path.segments.len() == 1 {
            return Some(p.path.segments[0].ident.to_string());
        }
    }
    None
}

/// Is this expr an untrusted-reader SOURCE (parse-provable vocabulary)?
fn is_source_expr(e: &Expr) -> bool {
    match e {
        Expr::Call(c) => {
            if let Expr::Path(p) = c.func.as_ref() {
                let segs: Vec<String> = p
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                // `TcpStream::connect(..)`, `reqwest::get(..)` — LAST TWO
                // segments (the fully-qualified `std::net::TcpStream::connect`
                // form is three).
                if segs.len() >= 2
                    && matches!(
                        (segs[segs.len() - 2].as_str(), segs[segs.len() - 1].as_str()),
                        ("TcpStream", "connect") | ("reqwest", "get")
                    )
                {
                    return true;
                }
            }
            c.args.iter().any(is_source_expr)
        }
        Expr::MethodCall(m) => {
            // `listener.accept()`, `listener.incoming()`,
            // `resp.bytes_stream()` — the network-reader vocabulary.
            if matches!(
                m.method.to_string().as_str(),
                "accept" | "incoming" | "bytes_stream"
            ) {
                return true;
            }
            // Transparent result-unwrapping between the source and the
            // binding: `listener.accept()?`, `.ok_or(..)?`, `..unwrap()`.
            if matches!(
                m.method.to_string().as_str(),
                "ok_or" | "ok_or_else" | "unwrap" | "unwrap_or" | "unwrap_or_else" | "expect" | "map_err" | "flatten"
            ) {
                return is_source_expr(&m.receiver);
            }
            is_source_expr(&m.receiver)
        }
        Expr::Try(t) => is_source_expr(&t.expr),
        Expr::Paren(p) => is_source_expr(&p.expr),
        Expr::Cast(c) => is_source_expr(&c.expr),
        _ => false,
    }
}

/// A readable label for a direct source chain (the fire report's ident).
fn source_label(e: &Expr) -> Option<String> {
    match e {
        Expr::Call(c) => {
            if let Expr::Path(p) = c.func.as_ref() {
                let segs: Vec<String> = p
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                if segs.len() >= 2 && segs[segs.len() - 1] == "connect" {
                    return Some(segs[segs.len() - 2].clone());
                }
                if segs.len() >= 2 && segs[segs.len() - 2] == "reqwest" {
                    return Some("reqwest".to_string());
                }
            }
            c.args.iter().find_map(source_label)
        }
        Expr::MethodCall(m) => {
            if matches!(
                m.method.to_string().as_str(),
                "accept" | "incoming" | "bytes_stream"
            ) {
                return Some(m.method.to_string());
            }
            source_label(&m.receiver).or_else(|| {
                m.args
                    .iter()
                    .find_map(source_label)
                    .or_else(|| source_label(&m.receiver))
            })
        }
        Expr::Try(t) => source_label(&t.expr),
        Expr::Paren(p) => source_label(&p.expr),
        _ => None,
    }
}

// ── pass 2: the sink walk ────────────────────────────────────────────────

fn scan_block_sinks(
    source: &str,
    block: &syn::Block,
    state: &ReadState,
    findings: &mut Vec<UnboundedReadFinding>,
) {
    scan_stmts_sinks(source, &block.stmts, state, findings);
}

fn scan_stmts_sinks(
    source: &str,
    stmts: &[Stmt],
    state: &ReadState,
    findings: &mut Vec<UnboundedReadFinding>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_expr_sinks(source, &init.expr, state, findings);
                }
            }
            Stmt::Item(item) => collect_in_item(source, item, findings),
            Stmt::Expr(e, _) => scan_expr_sinks(source, e, state, findings),
            Stmt::Macro(_) => {}
        }
    }
}

/// Shape A: does this expr denote `read_to_end`/`read_to_string` from a
/// tracked reader, un-bounded? Returns the report ident on a fire.
fn shape_a_fire(e: &Expr, state: &ReadState) -> Option<String> {
    match e {
        // `reader.read_to_end(&mut buf)?` — the method form.
        Expr::MethodCall(m) => {
            if !matches!(
                m.method.to_string().as_str(),
                "read_to_end" | "read_to_string"
            ) {
                return None;
            }
            // `.take(..)` ANYWHERE in the receiver chain = bounded.
            let mut cur: &Expr = m.receiver.as_ref();
            while let Expr::MethodCall(inner) = cur {
                if inner.method == "take" {
                    return None;
                }
                cur = inner.receiver.as_ref();
            }
            if let Some(id) = strip_refs_to_ident(cur) {
                if state.untrusted.contains(&id) {
                    return Some(id);
                }
                return None;
            }
            // Direct source chain: `TcpStream::connect(..)?.read_to_end(..)`.
            if is_source_expr(&m.receiver) {
                return source_label(&m.receiver);
            }
            None
        }
        // `std::io::Read::read_to_end(&mut reader, &mut buf)?` — the UFCS
        // form (import-free code and the corpus fixtures both use it).
        Expr::Call(c) => {
            let Expr::Path(p) = c.func.as_ref() else {
                return None;
            };
            if !p.path.segments.last().is_some_and(|s| {
                matches!(s.ident.to_string().as_str(), "read_to_end" | "read_to_string")
            }) {
                return None;
            }
            let first = c.args.first()?;
            if let Some(id) = strip_refs_to_ident(first) {
                return state
                    .untrusted
                    .contains(&id)
                    .then_some(id);
            }
            if is_source_expr(first) {
                return source_label(first);
            }
            None
        }
        _ => None,
    }
}

/// Strip `&`/`&mut`/parens/`?` down to a bare path ident.
fn strip_refs_to_ident(mut e: &Expr) -> Option<String> {
    loop {
        match e {
            Expr::Reference(r) => e = r.expr.as_ref(),
            Expr::Paren(p) => e = p.expr.as_ref(),
            Expr::Try(t) => e = t.expr.as_ref(),
            Expr::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 => {
                return Some(p.path.segments[0].ident.to_string());
            }
            _ => return None,
        }
    }
}

fn scan_expr_sinks(
    source: &str,
    expr: &Expr,
    state: &ReadState,
    findings: &mut Vec<UnboundedReadFinding>,
) {
    // Shape B first: loops drain when they read AND extend.
    match expr {
        Expr::Loop(l) => {
            scan_loop_body(source, &l.body.stmts, state, findings);
            return;
        }
        Expr::While(w) => {
            scan_expr_sinks(source, &w.cond, state, findings);
            scan_loop_body(source, &w.body.stmts, state, findings);
            return;
        }
        Expr::ForLoop(fl) => {
            scan_expr_sinks(source, &fl.expr, state, findings);
            scan_loop_body(source, &fl.body.stmts, state, findings);
            return;
        }
        _ => {}
    }

    // Shape A: the direct read site.
    if let Some(reader) = shape_a_fire(expr, state) {
        push_finding(source, expr, &reader, findings);
        return;
    }

    // Recurse in syntactic order (shape A inside non-loop control flow).
    match expr {
        Expr::If(i) => {
            scan_expr_sinks(source, &i.cond, state, findings);
            scan_block_sinks(source, &i.then_branch, state, findings);
            if let Some((_, else_e)) = &i.else_branch {
                scan_expr_sinks(source, else_e, state, findings);
            }
        }
        Expr::Match(m) => {
            scan_expr_sinks(source, &m.expr, state, findings);
            for arm in &m.arms {
                scan_expr_sinks(source, &arm.body, state, findings);
            }
        }
        Expr::Block(b) => scan_block_sinks(source, &b.block, state, findings),
        Expr::Unsafe(u) => scan_block_sinks(source, &u.block, state, findings),
        Expr::MethodCall(m) => {
            scan_expr_sinks(source, &m.receiver, state, findings);
            for a in &m.args {
                scan_expr_sinks(source, a, state, findings);
            }
        }
        Expr::Call(c) => {
            scan_expr_sinks(source, &c.func, state, findings);
            for a in &c.args {
                scan_expr_sinks(source, a, state, findings);
            }
        }
        Expr::Assign(a) => scan_expr_sinks(source, &a.right, state, findings),
        Expr::Binary(b) => {
            scan_expr_sinks(source, &b.left, state, findings);
            scan_expr_sinks(source, &b.right, state, findings);
        }
        // `read_to_end(..)?` — the Try wrapper must not hide the read.
        Expr::Try(t) => scan_expr_sinks(source, &t.expr, state, findings),
        _ => {}
    }
}

fn push_finding(
    source: &str,
    expr: &Expr,
    reader: &str,
    findings: &mut Vec<UnboundedReadFinding>,
) {
    let span = expr.span().byte_range();
    if span.end <= source.len() {
        findings.push(UnboundedReadFinding {
            read_text: source[span.clone()].to_string(),
            read_span: span,
            reader_ident: reader.to_string(),
            advice: ADVICE,
        });
    }
}

/// Shape B: a loop whose body reads from a tracked reader AND extends a
/// local Vec, with no cap marker. Caps: `resize` on the Vec (fn-wide),
/// `buf.len()` in a comparison inside the loop, or the READ-COUNT binding
/// (the local the read was assigned to) compared against a NON-ZERO
/// operand (`n == 0` is the EOF idiom, not a cap).
fn scan_loop_body(
    source: &str,
    stmts: &[Stmt],
    state: &ReadState,
    findings: &mut Vec<UnboundedReadFinding>,
) {
    let mut read_site: Option<(Range<usize>, String)> = None;
    let mut count_idents: Vec<String> = Vec::new();
    for stmt in stmts {
        if let Stmt::Local(l) = stmt {
            if let Some(init) = &l.init {
                if let Some(reader) = loop_read_site(&init.expr, state) {
                    count_idents = local_idents(&l.pat);
                    read_site = Some((init.expr.span().byte_range(), reader));
                }
            }
        }
    }
    let mut scout = LoopScout {
        state,
        count_idents: &count_idents,
        extended_vec: None,
        capped: false,
    };
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scout.visit_expr(&init.expr);
                }
            }
            Stmt::Expr(e, _) => scout.visit_expr(e),
            Stmt::Item(_) | Stmt::Macro(_) => {}
        }
    }

    let grows = read_site.is_some() && scout.extended_vec.is_some();
    let vec_cap = scout
        .extended_vec
        .as_ref()
        .is_some_and(|v| state.resized.contains(v));
    if grows && !scout.capped && !vec_cap {
        if let Some((span, reader)) = read_site {
            if span.end <= source.len() {
                findings.push(UnboundedReadFinding {
                    read_text: source[span.clone()].to_string(),
                    read_span: span,
                    reader_ident: reader,
                    advice: ADVICE,
                });
            }
        }
    }
    // Shape A still applies inside loops (a read_to_end in a for body is a
    // direct fire); the drain's own read/read_exact is NOT a shape-A form,
    // so this cannot double-report the drain site.
    scan_stmts_sinks(source, stmts, state, findings);
}

/// The loop's read half: a `read`/`read_exact` on a tracked reader
/// (method or UFCS form). Returns the report ident.
fn loop_read_site(e: &Expr, state: &ReadState) -> Option<String> {
    let mut found = None;
    let mut v = ReadScout {
        state,
        found: &mut found,
    };
    v.visit_expr(e);
    found
}

struct ReadScout<'a> {
    state: &'a ReadState,
    found: &'a mut Option<String>,
}

impl<'ast> Visit<'ast> for ReadScout<'_> {
    fn visit_expr(&mut self, e: &'ast Expr) {
        match e {
            Expr::MethodCall(m) => {
                if matches!(m.method.to_string().as_str(), "read" | "read_exact") {
                    if let Some(id) = strip_refs_to_ident(&m.receiver) {
                        if self.state.untrusted.contains(&id) && self.found.is_none() {
                            *self.found = Some(id);
                        }
                    }
                }
            }
            Expr::Call(c) => {
                if let Expr::Path(p) = c.func.as_ref() {
                    if p.path.segments.last().is_some_and(|s| {
                        matches!(s.ident.to_string().as_str(), "read" | "read_exact")
                    }) {
                        if let Some(first) = c.args.first() {
                            if let Some(id) = strip_refs_to_ident(first) {
                                if self.state.untrusted.contains(&id) && self.found.is_none() {
                                    *self.found = Some(id);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        syn::visit::visit_expr(self, e);
    }
}

struct LoopScout<'a> {
    state: &'a ReadState,
    count_idents: &'a [String],
    extended_vec: Option<String>,
    capped: bool,
}

impl<'ast> Visit<'ast> for LoopScout<'_> {
    fn visit_expr(&mut self, e: &'ast Expr) {
        match e {
            Expr::MethodCall(m) => {
                let method = m.method.to_string();
                if matches!(method.as_str(), "extend" | "extend_from_slice") {
                    if let Some(id) = path_ident(&m.receiver) {
                        if self.state.vecs.contains(&id) {
                            self.extended_vec = Some(id);
                        }
                    }
                }
                if method == "len" {
                    if let Some(id) = path_ident(&m.receiver) {
                        if self.state.vecs.contains(&id) {
                            self.capped = true;
                        }
                    }
                }
            }
            Expr::Binary(b) => {
                if matches!(
                    b.op,
                    BinOp::Lt(_) | BinOp::Gt(_) | BinOp::Le(_) | BinOp::Ge(_) | BinOp::Eq(_) | BinOp::Ne(_)
                ) {
                    let lhs_count = path_ident(&b.left).is_some_and(|id| self.count_idents.contains(&id));
                    let rhs_count = path_ident(&b.right).is_some_and(|id| self.count_idents.contains(&id));
                    let lhs_zero = is_zero_literal(&b.left);
                    let rhs_zero = is_zero_literal(&b.right);
                    if (lhs_count && !rhs_zero) || (rhs_count && !lhs_zero) {
                        self.capped = true;
                    }
                }
            }
            _ => {}
        }
        syn::visit::visit_expr(self, e);
    }
}

fn is_zero_literal(e: &Expr) -> bool {
    matches!(e, Expr::Lit(l) if matches!(&l.lit, syn::Lit::Int(i) if
        i.base10_parse::<u64>().is_ok_and(|v| v == 0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(src: &str) -> Vec<UnboundedReadFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    // ── positives ────────────────────────────────────────────────────────

    #[test]
    fn batch140_listener_child_read_to_end_fires() {
        // The canonical accept()-child drain (the h2-class shape).
        let src = "\nuse std::io::Read;\n\
            pub fn handle(listener: &std::net::TcpListener) -> std::io::Result<Vec<u8>> {\n\
                let (mut stream, _addr) = listener.accept()?;\n\
                let mut buf = Vec::new();\n\
                stream.read_to_end(&mut buf)?;\n\
                Ok(buf)\n\
            }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the accept()-child read_to_end must fire");
        assert_eq!(f[0].reader_ident, "stream");
        assert!(f[0].advice.contains("take"));
    }

    #[test]
    fn batch140_param_typed_stream_ufcs_fires() {
        // The corpus fixture's form: a TcpStream-typed param read via the
        // import-free UFCS call.
        let src = "\n\
            pub fn read_request(stream: std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n\
                let mut buf = Vec::new();\n\
                std::io::Read::read_to_end(&mut stream, &mut buf)?;\n\
                Ok(buf)\n\
            }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the TcpStream-typed param drain must fire");
        assert_eq!(f[0].reader_ident, "stream");
        assert!(f[0].read_text.contains("read_to_end"));
    }

    #[test]
    fn batch140_direct_connect_chain_fires() {
        // No binding at all: the source call IS the receiver.
        let src = "\nuse std::io::Read;\n\
            pub fn probe(addr: &str) -> std::io::Result<Vec<u8>> {\n\
                let mut buf = Vec::new();\n\
                std::net::TcpStream::connect(addr)?.read_to_end(&mut buf)?;\n\
                Ok(buf)\n\
            }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the direct connect chain must fire");
        assert_eq!(f[0].reader_ident, "TcpStream");
    }

    #[test]
    fn batch140_loop_extend_unbounded_fires() {
        // The loop-drain shape: read into a chunk, extend a growable Vec,
        // the only comparison is the EOF sentinel (n == 0).
        let src = "\n\
            pub fn drain(stream: std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n\
                let mut buf = Vec::new();\n\
                let mut chunk = [0_u8; 4096];\n\
                loop {\n\
                    let n = std::io::Read::read(&mut stream, &mut chunk)?;\n\
                    if n == 0 { break; }\n\
                    buf.extend_from_slice(&chunk[..n]);\n\
                }\n\
                Ok(buf)\n\
            }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the unbounded loop drain must fire");
        assert_eq!(f[0].reader_ident, "stream");
    }

    #[test]
    fn batch140_incoming_for_loop_fires() {
        let src = "\nuse std::io::Read;\n\
            pub fn serve(listener: &std::net::TcpListener) {\n\
                for stream in listener.incoming().flatten() {\n\
                    let mut buf = Vec::new();\n\
                    let _ = stream.read_to_end(&mut buf);\n\
                }\n\
            }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the incoming()-loop read must fire");
    }

    #[test]
    fn batch140_finding_spans_slice_the_source() {
        let src = "\n\
            pub fn read_request(stream: std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n\
                let mut buf = Vec::new();\n\
                std::io::Read::read_to_end(&mut stream, &mut buf)?;\n\
                Ok(buf)\n\
            }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1);
        assert_eq!(
            f[0].read_text,
            "std::io::Read::read_to_end(&mut stream, &mut buf)"
        );
    }

    // ── zero-FP negative suite ───────────────────────────────────────

    #[test]
    fn batch140_take_chain_bounded_twin_is_silent() {
        // The bounded twin: take anywhere in the receiver chain.
        let src = "\nuse std::io::Read;\n\
            pub fn handle(listener: &std::net::TcpListener) -> std::io::Result<Vec<u8>> {\n\
                let (mut stream, _) = listener.accept()?;\n\
                let mut buf = Vec::new();\n\
                stream.take(1 << 20).read_to_end(&mut buf)?;\n\
                Ok(buf)\n\
            }\n";
        assert!(findings(src).is_empty(), "take() in the chain must suppress");
    }

    #[test]
    fn batch140_take_binding_ufcs_form_is_silent() {
        // The corpus fixed form: take produces a fresh (untracked) reader.
        let src = "\n\
            pub fn read_request(stream: std::net::TcpStream, max: u64) -> std::io::Result<Vec<u8>> {\n\
                let mut limited = std::io::Read::take(stream, max);\n\
                let mut buf = Vec::with_capacity(max as usize);\n\
                std::io::Read::read_to_end(&mut limited, &mut buf)?;\n\
                Ok(buf)\n\
            }\n";
        assert!(findings(src).is_empty(), "the take-bound UFCS form must not fire");
    }

    #[test]
    fn batch140_file_reader_is_silent() {
        // The trusted regime: local files never track.
        let src = "\nuse std::io::Read;\n\
            pub fn load_config(path: &std::path::Path) -> std::io::Result<Vec<u8>> {\n\
                let mut f = std::fs::File::open(path)?;\n\
                let mut buf = Vec::new();\n\
                f.read_to_end(&mut buf)?;\n\
                Ok(buf)\n\
            }\n";
        assert!(findings(src).is_empty(), "File readers are the trusted regime");
    }

    #[test]
    fn batch140_unknown_slice_reader_is_silent() {
        // `&payload[..]` is an unknown receiver — the honest NO.
        let src = "\nuse std::io::Read;\n\
            pub fn parse(payload: &[u8]) -> std::io::Result<Vec<u8>> {\n\
                let mut buf = Vec::new();\n\
                (&payload[..]).read_to_end(&mut buf)?;\n\
                Ok(buf)\n\
            }\n";
        assert!(findings(src).is_empty(), "unknown readers must not fire");
    }

    #[test]
    fn batch140_with_capacity_without_read_is_silent() {
        // No read call at all — no sink, no match.
        let src = "\n\
            pub fn build(stream: std::net::TcpStream) -> Vec<u8> {\n\
                let _ = stream;\n\
                Vec::with_capacity(4096)\n\
            }\n";
        assert!(findings(src).is_empty());
    }

    #[test]
    fn batch140_loop_resize_capped_is_silent() {
        // The capacity-capped drain: the Vec is pre-sized with resize()
        // after a bound check — a different, bounded program.
        let src = "\n\
            pub fn capped(stream: std::net::TcpStream, max: usize) -> std::io::Result<Vec<u8>> {\n\
                let mut buf = Vec::new();\n\
                let mut chunk = [0_u8; 4096];\n\
                if max == 0 || max > (1 << 20) { return Err(std::io::ErrorKind::InvalidData.into()); }\n\
                buf.resize(max, 0);\n\
                loop {\n\
                    let n = std::io::Read::read(&mut stream, &mut chunk)?;\n\
                    if n == 0 { break; }\n\
                    buf.extend_from_slice(&chunk[..n]);\n\
                }\n\
                Ok(buf)\n\
            }\n";
        assert!(findings(src).is_empty(), "resize()-capped drains must not fire");
    }

    #[test]
    fn batch140_loop_len_cap_is_silent() {
        // A buf.len() comparison inside the loop is a cap.
        let src = "\n\
            pub fn capped(stream: std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n\
                const MAX: usize = 1 << 20;\n\
                let mut buf = Vec::new();\n\
                let mut chunk = [0_u8; 4096];\n\
                loop {\n\
                    let n = std::io::Read::read(&mut stream, &mut chunk)?;\n\
                    if n == 0 { break; }\n\
                    if buf.len() >= MAX { break; }\n\
                    buf.extend_from_slice(&chunk[..n]);\n\
                }\n\
                Ok(buf)\n\
            }\n";
        assert!(findings(src).is_empty(), "buf.len() cap must suppress");
    }

    #[test]
    fn batch140_loop_counter_cap_still_fires_review_decides() {
        // The DOCUMENTED recall limit: a separate chunk-counter cap
        // (`seen > MAX_CHUNKS`) is a bounded program our structural caps
        // (resize / buf.len() / read-count comparison) do not recognize —
        // the chunk size itself is fixed here, so review decides.
        // Suggest-only is the blast-radius control (the length-prefix
        // trusted-length precedent). The read-count BINDING cap (n vs a
        // non-zero operand) IS recognized — see the EOF test below.
        let src = "\n\
            pub fn capped(stream: std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n\
                const MAX_CHUNKS: u64 = 256;\n\
                let mut buf = Vec::new();\n\
                let mut chunk = [0_u8; 4096];\n\
                let mut seen: u64 = 0;\n\
                loop {\n\
                    let n = std::io::Read::read(&mut stream, &mut chunk)?;\n\
                    if n == 0 { break; }\n\
                    if seen > MAX_CHUNKS { break; }\n\
                    seen += 1;\n\
                    buf.extend_from_slice(&chunk[..n]);\n\
                }\n\
                Ok(buf)\n\
            }\n";
        assert_eq!(findings(src).len(), 1, "counter caps fire; review decides");
    }

    #[test]
    fn batch140_eof_zero_check_does_not_suppress_and_count_cap_does() {
        // Pinned together: `n == 0` is EOF (no cap — the drain fires) while
        // `n > MAX_CHUNKS` on the READ-COUNT binding IS a cap (silent).
        let drain = "\n\
            pub fn drain(stream: std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n\
                let mut buf = Vec::new();\n\
                let mut chunk = [0_u8; 4096];\n\
                loop {\n\
                    let n = std::io::Read::read(&mut stream, &mut chunk)?;\n\
                    if n == 0 { break; }\n\
                    buf.extend_from_slice(&chunk[..n]);\n\
                }\n\
                Ok(buf)\n\
            }\n";
        assert_eq!(findings(drain).len(), 1);
        let capped = "\n\
            pub fn drain(stream: std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n\
                const MAX_CHUNKS: u64 = 64;\n\
                let mut buf = Vec::new();\n\
                let mut chunk = [0_u8; 4096];\n\
                loop {\n\
                    let n = std::io::Read::read(&mut stream, &mut chunk)?;\n\
                    if n == 0 { break; }\n\
                    if n > MAX_CHUNKS { break; }\n\
                    buf.extend_from_slice(&chunk[..n]);\n\
                }\n\
                Ok(buf)\n\
            }\n";
        assert!(
            findings(capped).is_empty(),
            "read-count vs non-zero const is a recognized cap"
        );
    }

    #[test]
    fn batch140_bare_read_param_is_silent() {
        // The honest recall limit: `R: Read` proves nothing about trust.
        let src = "\nuse std::io::Read;\n\
            pub fn load<R: Read>(mut reader: R) -> std::io::Result<Vec<u8>> {\n\
                let mut buf = Vec::new();\n\
                reader.read_to_end(&mut buf)?;\n\
                Ok(buf)\n\
            }\n";
        assert!(findings(src).is_empty(), "generic readers are an honest NO");
    }

    #[test]
    fn batch140_test_mod_is_skipped() {
        // House discipline: cfg(test) mods are fixture habitat.
        let src = "\nuse std::io::Read;\n\
            #[cfg(test)]\n\
            mod wire_tests {\n\
                pub fn drain(stream: std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n\
                    let mut buf = Vec::new();\n\
                    std::io::Read::read_to_end(&mut stream, &mut buf)?;\n\
                    Ok(buf)\n\
                }\n\
            }\n";
        assert!(findings(src).is_empty(), "cfg(test) mods must be skipped");
    }

    #[test]
    fn batch140_advice_names_the_class() {
        assert!(ADVICE.contains("take"));
        assert!(ADVICE.contains("RUSTSEC-2024-0003"));
    }
}

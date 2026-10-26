//! `client-ip-from-untrusted-forwarded-header` — SUGGEST-ONLY detector
//! (Batch 292, the hevnsnt/`CVEngine` distill; Research 330 / Plan 296).
//!
//! An `x-forwarded-for` header read used as the client identity (the
//! rate-limit key, ban key, audit actor) without any trusted-proxy gate:
//! the header is attacker-settable, so the leftmost (or unconditional
//! whole-header) read lets any caller rotate the identity a throttle or
//! ban decision keys on. `CVEngine`'s `client_ip.py` is the textbook-correct
//! resolver and the rule's decision content: no trusted proxies → the
//! header is ignored entirely; the peer is a trusted proxy → walk the
//! chain right-to-left, first untrusted hop wins; a malformed hop → the
//! whole chain is untrusted → fail closed to the peer.
//!
//! ## Why suggest-only (never a fix)
//!
//! The trusted-proxy set is deployment policy (which proxy networks are
//! trusted is the operator's answer); a mechanical rewrite cannot know it.
//! This module ships a parse-only DETECTION walk only: NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm — the suggestion surfaces through
//! retrieval (`--suggest` / the corpus entry) and review.
//!
//! ## The bounded form
//!
//! ```text
//! // no trusted proxies → peer wins
//! if !trusted(peer) { return peer; }
//! // peer trusted → right-to-left walk, first untrusted hop
//! for hop in chain.split(',').rev() { ... }
//! // malformed hop → fail closed to the peer
//! let Ok(ip) = hop.trim().parse() else { return peer; };
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Header-literal gate (the FP control)**: the fn must carry the
//!   `x-forwarded-for` header name as a STRING LITERAL (case-insensitive;
//!   the `Forwarded` header's `for=` form is a different, rarely-deployed
//!   surface and stays silent). Identifiers named after the header are an
//!   honest NO.
//! - **Identity/throttle vocabulary co-occurrence**: the fn's token text
//!   must also carry an identity/rate-limit use (`client_ip` / `rate_limit` /
//!   ratelimit / throttle / identity / audit / banned). An xff read with
//!   no identity use (logging, debug echo) stays silent.
//! - **Trusted-marker suppression (fn-wide)**: any `trusted` token in the
//!   fn suppresses — a proxy-aware resolver (the fixed shape) mentions
//!   its trust set, and `untrusted` implies the same awareness. One
//!   written word covers both the resolver and its helpers-in-fn.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped;
//!   macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - The identity vocabulary proves a USE CLASS, not a decision — an
//!   audit-log fn fires too, and review decides.
//! - Framework extractors that never name the literal
//!   (`axum::extract::ConnectInfo`, `XForwardedFor` newtypes) are out of
//!   scope — the string-literal gate is the parse-provable surface.
//! - Cross-fn identity flow (read in one fn, keyed in another) undetected.
//! - Batch 292 / Plan 296 / Research 330 (hevnsnt/`CVEngine` @ `9dd9c441`,
//!   GPL-3.0 — cited as class provenance; the fixture is an original
//!   synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item};

/// One detected untrusted-forwarded-header identity site.
#[derive(Debug, Clone)]
pub struct ClientIpForwardedFinding {
    /// Byte range of the header-literal expression that tripped the gate.
    pub site_span: Range<usize>,
    /// The literal expression's source text.
    pub site_text: String,
    /// The advice line (the three-regime resolver + the class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "do not key identity on `x-forwarded-for` without a \
trusted-proxy gate: the header is attacker-settable — ignore it unless the peer \
is a trusted proxy; then walk right-to-left and take the FIRST untrusted hop; a \
malformed hop fails closed to the peer (Batch 292, suggest-only)";

/// The identity/throttle vocabulary (token-text substrings, lowercase).
const IDENTITY_TOKENS: &[&str] = &[
    "client_ip",
    "clientip",
    "rate_limit",
    "ratelimit",
    "throttle",
    "identity",
    "audit",
    "banned",
];

/// The fn-wide suppression marker: a proxy-aware resolver mentions its
/// trust set (`untrusted` implies the same awareness).
const SUPPRESS_MARKERS: &[&str] = &["trusted"];

/// Detect untrusted forwarded-header identity reads (suggest-only; no
/// edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<ClientIpForwardedFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<ClientIpForwardedFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(source, &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production identity resolution.
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

fn scan_fn(source: &str, block: &syn::Block, findings: &mut Vec<ClientIpForwardedFinding>) {
    // Gate 2/3 first (fn-wide token text, lowercase): the identity use and
    // the trusted-marker suppression.
    let fn_ts = quote::ToTokens::to_token_stream(block).to_string();
    let fn_lower = fn_ts.to_ascii_lowercase();
    if !IDENTITY_TOKENS.iter().any(|t| fn_lower.contains(t)) {
        return; // no identity/throttle use: a bare header read stays silent
    }
    if SUPPRESS_MARKERS.iter().any(|m| fn_lower.contains(m)) {
        return; // proxy-aware (the fixed shape): silent
    }
    // Gate 1: the header name as a STRING LITERAL anywhere in the fn.
    if let Some((span, text)) = header_literal_span(block, source) {
        findings.push(ClientIpForwardedFinding {
            site_text: text,
            site_span: span,
            advice: ADVICE,
        });
    }
}

/// The first string literal in the fn whose lowercase value carries the
/// `x-forwarded-for` header name (dash/underscore-insensitive — the
/// `x_forwarded_for` config-key spelling names the same header).
fn header_literal_span(block: &syn::Block, source: &str) -> Option<(Range<usize>, String)> {
    let mut found = None;
    walk_expr_for_header(&block.stmts, source, &mut found);
    found
}

fn walk_stmts_for_header(
    stmts: &[syn::Stmt],
    source: &str,
    found: &mut Option<(Range<usize>, String)>,
) {
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    walk_expr_for_header_expr(&init.expr, source, found);
                }
            }
            syn::Stmt::Item(item) => walk_item_for_header(item, source, found),
            syn::Stmt::Expr(e, _) => walk_expr_for_header_expr(e, source, found),
            syn::Stmt::Macro(_) => {}
        }
    }
}

fn walk_item_for_header(item: &Item, source: &str, found: &mut Option<(Range<usize>, String)>) {
    match item {
        Item::Fn(f) => walk_stmts_for_header(&f.block.stmts, source, found),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    walk_stmts_for_header(&m.block.stmts, source, found);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    walk_item_for_header(nested, source, found);
                }
            }
        }
        _ => {}
    }
}

fn walk_expr_for_header(
    stmts: &[syn::Stmt],
    source: &str,
    found: &mut Option<(Range<usize>, String)>,
) {
    walk_stmts_for_header(stmts, source, found);
}

fn walk_expr_for_header_expr(e: &Expr, source: &str, found: &mut Option<(Range<usize>, String)>) {
    if found.is_some() {
        return;
    }
    match e {
        Expr::Lit(l) => {
            if let syn::Lit::Str(s) = &l.lit {
                let v = s.value().to_ascii_lowercase().replace('_', "-");
                if v.contains("x-forwarded-for") {
                    let span = e.span().byte_range();
                    if span.end <= source.len() {
                        let text = source[span.clone()].to_string();
                        *found = Some((span, text));
                    }
                }
            }
        }
        Expr::MethodCall(m) => {
            walk_expr_for_header_expr(&m.receiver, source, found);
            for a in &m.args {
                walk_expr_for_header_expr(a, source, found);
            }
        }
        Expr::Call(c) => {
            walk_expr_for_header_expr(&c.func, source, found);
            for a in &c.args {
                walk_expr_for_header_expr(a, source, found);
            }
        }
        Expr::If(i) => {
            walk_expr_for_header_expr(&i.cond, source, found);
            walk_stmts_for_header(&i.then_branch.stmts, source, found);
            if let Some((_, else_e)) = &i.else_branch {
                walk_expr_for_header_expr(else_e, source, found);
            }
        }
        Expr::Match(m) => {
            walk_expr_for_header_expr(&m.expr, source, found);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    walk_expr_for_header_expr(guard, source, found);
                }
                walk_expr_for_header_expr(&arm.body, source, found);
            }
        }
        Expr::Block(b) => walk_stmts_for_header(&b.block.stmts, source, found),
        Expr::Unsafe(u) => walk_stmts_for_header(&u.block.stmts, source, found),
        Expr::Loop(l) => walk_stmts_for_header(&l.body.stmts, source, found),
        Expr::While(w) => {
            walk_expr_for_header_expr(&w.cond, source, found);
            walk_stmts_for_header(&w.body.stmts, source, found);
        }
        Expr::ForLoop(fl) => {
            walk_expr_for_header_expr(&fl.expr, source, found);
            walk_stmts_for_header(&fl.body.stmts, source, found);
        }
        Expr::Try(t) => walk_expr_for_header_expr(&t.expr, source, found),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::ClientIpForwardedFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 292 gate — the corpus `buggy_code` MUST fire.
    #[test]
    fn batch292_leftmost_read_as_identity_fires() {
        let src = "use std::net::IpAddr;\n\n\
             pub fn client_ip(forwarded: Option<&str>, peer: IpAddr) -> IpAddr {\n    \
             forwarded\n        \
             .and_then(|chain| chain.split(',').next())\n        \
             .and_then(|hop| hop.trim().parse::<IpAddr>().ok())\n        \
             .unwrap_or(peer)\n}\n";
        assert!(
            findings(src).is_empty(),
            "no header literal + no identity use: silent"
        );
        // Now the identity-keyed form with the literal: fires.
        let src = "use std::net::IpAddr;\n\n\
             pub fn throttle_key(req: &Request) -> IpAddr {\n    \
             let ip = req.header(\"x-forwarded-for\")\n        \
             .and_then(|c| c.split(',').next())\n        \
             .and_then(|h| h.trim().parse::<IpAddr>().ok());\n    \
             if banned(ip) { panic!(\"rejected\"); }\n    \
             ip.unwrap()\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The fixed resolver (a trusted marker present) stays silent.
    #[test]
    fn trusted_resolver_silent() {
        let src = "use std::net::IpAddr;\n\n\
             pub fn client_ip(forwarded: Option<&str>, peer: IpAddr) -> IpAddr {\n    \
             if !trusted_proxy(peer) { return peer; }\n    \
             let chain = forwarded.unwrap_or_default();\n    \
             for hop in chain.split(',').rev() {\n        \
             let Ok(ip) = hop.trim().parse::<IpAddr>() else { return peer; };\n        \
             if !trusted_proxy(ip) { return ip; }\n    \
             }\n    \
             peer\n}\n";
        assert!(findings(src).is_empty(), "trusted marker: silent");
    }

    /// An xff read with no identity/throttle use stays silent.
    #[test]
    fn logging_read_silent() {
        let src = "pub fn debug_headers(req: &Request) -> String {\n    \
             let xff = req.header(\"X-Forwarded-For\").unwrap_or_default();\n    \
             format!(\"xff={xff}\")\n}\n";
        assert!(findings(src).is_empty(), "no identity vocabulary: silent");
    }

    /// A different header with identity vocabulary stays silent — the
    /// literal gate is the FP control.
    #[test]
    fn other_header_silent() {
        let src = "pub fn audit_key(req: &Request) -> String {\n    \
             req.header(\"x-request-id\").unwrap_or_default().to_owned()\n}\n";
        assert!(findings(src).is_empty());
    }

    /// The underscore config-key spelling names the same header (fires).
    #[test]
    fn underscore_spelling_fires() {
        let src = "pub fn identity_of(cfg: &Config) -> String {\n    \
             let client_ip = cfg.get(\"x_forwarded_for\").cloned().unwrap_or_default();\n    \
             client_ip\n}\n";
        assert_eq!(findings(src).len(), 1);
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn t() {\n        \
             let _ = header(\"x-forwarded-for\");\n    \
             }\n}\n";
        assert!(findings(src).is_empty());
    }
}

//! `unvalidated-egress-url` — SUGGEST-ONLY detector
//! (Batch 258, the cloudflare/security-audit-skill distill; Research 292 /
//! Plan 262).
//!
//! A URL assembled from request/tenant/config input reaching an outbound
//! client sink (`reqwest::get(u)`, `ureq::get(u)`) with no scheme/host
//! allowlist and no private-range refusal lets the caller pick the egress
//! target: cloud metadata endpoints, loopback admin edges, internal peers
//! — the SSRF class ("attacker picks the egress target"). The bounded
//! repair is policy, never mechanical: pin the scheme, check the host
//! against an allowlist, refuse private/loopback/link-local ranges.
//!
//! ## Why suggest-only (never a fix)
//!
//! The allowlist is deployment policy (which hosts are legitimate is the
//! operator's answer); a mechanical rewrite cannot invent it. This module
//! ships a parse-only DETECTION walk only: NO `FIXABLE_SEC` membership,
//! NO `collect_edits` arm — the suggestion surfaces through retrieval
//! (`--suggest` / the corpus entry) and review.
//!
//! ## The bounded form
//!
//! ```text
//! if !ALLOWED_HOSTS.contains(&host) { return Err(..) }   // + scheme pin
//! let url = format!("https://{host}/report/{probe}");
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Named-sink vocabulary, parse-provable only**: free calls under the
//!   `reqwest::` / `ureq::` crate prefixes (`reqwest::get`,
//!   `reqwest::blocking::get`, `ureq::post`, ...). Client-binding method
//!   sinks (`client.get(u)`) are untypeable parse-only — an honest NO.
//! - **URL-origin vocabulary, parse-provable only**: the URL argument is a
//!   fn parameter whose name carries a URL token (`url` / `uri` / `host` /
//!   `endpoint` / `target` / `addr` / `hostname`), or a local bound from a
//!   `format!` WITH a placeholder (an interpolated URL — something dynamic
//!   went in). String-literal URLs never fire.
//! - **Validation vocabulary suppresses fn-wide** (one guarded fn hides a
//!   sibling — the house conservative rule): `allowlist` / `allow_list` /
//!   `allowed`, `scheme()` / `host_str()`, `validate_url` / `valid_url` /
//!   `check_url`, `is_safe` / `safe_host`, `private` / `loopback` /
//!   `link_local` anywhere in the fn → silent.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped;
//!   macro tokens are never descended into (the `format!` ARGUMENT scan is
//!   a token-text placeholder check, not a descent).
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Framework-shaped extraction (an axum `Path<String>` / `Query` reaching
//!   the sink through a binding) is an honest NO — the same
//!   untrusted-source thinness `unbounded-read-into-vec` discloses; only
//!   the parameter vocabulary is parse-provable.
//! - The vocabulary proves an INPUT CLASS, not attacker control — an
//!   internal-only tool fetching its own config URL fires too, and review
//!   decides. Suggest-only is the blast-radius control.
//! - A bare scheme check (`starts_with("https")`) does NOT suppress — the
//!   host is the SSRF axis; the advice still applies. The full guard
//!   vocabulary above does.
//! - Cross-fn URL construction (built in a helper, sunk in the caller) is
//!   not connected parse-only.
//! - **Sink-level novelty over `unbounded-read-into-vec` (owned)**: that
//!   rule owns `reqwest::get(..)` results as untrusted READ sources
//!   (response bytes growing a buffer); this owns the URL-CHOICE side of
//!   the same call (which host the egress aims at) — the Plan 156 T2
//!   sink-distinct adjudication shape.
//! - Batch 258 / Plan 262 / Research 292 (cloudflare/security-audit-skill
//!   @ `c1c8a8c`, MIT — cited as class provenance; the fixture is an
//!   original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item, Stmt};

/// One detected unvalidated egress fetch.
#[derive(Debug, Clone)]
pub struct EgressUrlFinding {
    /// Byte range of the egress sink call.
    pub sink_span: Range<usize>,
    /// The sink call's source text.
    pub sink_text: String,
    /// The URL origin (the parameter name or the `format!` label).
    pub url_origin: String,
    /// The advice line (allowlist + private-range refusal + the class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "validate the egress target before the fetch: pin the \
scheme and check the host against an allowlist, and refuse \
private/loopback/link-local ranges — a URL assembled from request or tenant \
input lets the caller aim the egress at cloud metadata, loopback admin edges, \
or internal peers (SSRF; Batch 258, suggest-only)";

/// The fn-wide validation spellings (lowercase substring scan — one
/// guarded fn hides a sibling).
const GUARD_MARKERS: &[&str] = &[
    "allowlist",
    "allow_list",
    "allowed",
    "scheme()",
    "host_str()",
    "validate_url",
    "valid_url",
    "check_url",
    "is_safe",
    "safe_host",
    "private",
    "loopback",
    "link_local",
];

/// The outbound client crate prefixes (first path segment).
const EGRESS_CRATES: &[&str] = &["reqwest", "ureq"];

/// URL-vocabulary tokens for parameter names.
const URL_TOKENS: &[&str] = &[
    "url", "uri", "host", "hostname", "endpoint", "target", "addr",
];

/// Detect unvalidated egress fetches (suggest-only; no edits are produced
/// by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<EgressUrlFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<EgressUrlFinding>) {
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
            // fixture habitat, not production egress paths.
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

fn scan_fn(source: &str, f: &syn::ItemFn, findings: &mut Vec<EgressUrlFinding>) {
    scan_shaped(source, &f.sig, &f.block, findings);
}

fn scan_shaped(
    source: &str,
    sig: &syn::Signature,
    block: &syn::Block,
    findings: &mut Vec<EgressUrlFinding>,
) {
    // fn-wide guard gate (token-text scan, space-stripped + lowercased —
    // token streams render with separating spaces and constants are
    // conventionally SCREAMING_CASE).
    let sig_ts = quote::ToTokens::to_token_stream(sig).to_string();
    let block_ts = quote::ToTokens::to_token_stream(block).to_string();
    let fn_text = format!("{sig_ts}{block_ts}")
        .replace(' ', "")
        .to_ascii_lowercase();
    if GUARD_MARKERS.iter().any(|m| fn_text.contains(m)) {
        return;
    }

    // URL-origin vocabulary: params + format!-built locals.
    let mut url_params: std::collections::HashSet<String> = std::collections::HashSet::new();
    for input in &sig.inputs {
        if let syn::FnArg::Typed(pt) = input {
            let mut pats = Vec::new();
            collect_pat_idents(&pt.pat, &mut pats);
            for p in pats {
                if has_url_token(&p) {
                    url_params.insert(p);
                }
            }
        }
    }
    let mut url_locals: std::collections::HashSet<String> = std::collections::HashSet::new();
    collect_url_locals(block, &mut url_locals);
    if url_params.is_empty() && url_locals.is_empty() {
        return;
    }

    scan_sinks(source, block, &url_params, &url_locals, findings);
}

fn has_url_token(name: &str) -> bool {
    name.to_ascii_lowercase()
        .split(['_', '-', ' ', '.'])
        .any(|t| URL_TOKENS.contains(&t))
}

/// Locals bound (transparently) from a `format!` WITH a placeholder.
fn collect_url_locals(block: &syn::Block, out: &mut std::collections::HashSet<String>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    if is_interpolated_format(&init.expr) {
                        let mut pats = Vec::new();
                        collect_pat_idents(&l.pat, &mut pats);
                        for p in pats {
                            out.insert(p);
                        }
                    }
                    // Transparent re-binding of a tracked local keeps it.
                    if let Expr::Path(p) = strip_refs(&init.expr) {
                        if p.qself.is_none() && p.path.segments.len() == 1 {
                            let id = p.path.segments[0].ident.to_string();
                            if out.contains(&id) {
                                let mut pats = Vec::new();
                                collect_pat_idents(&l.pat, &mut pats);
                                for p in pats {
                                    out.insert(p);
                                }
                            }
                        }
                    }
                }
            }
            Stmt::Item(item) => collect_url_locals_item(item, out),
            Stmt::Expr(e, _) => collect_url_locals_expr(e, out),
            Stmt::Macro(_) => {}
        }
    }
}

fn collect_url_locals_item(item: &Item, out: &mut std::collections::HashSet<String>) {
    match item {
        Item::Fn(f) => collect_url_locals(&f.block, out),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    collect_url_locals(&m.block, out);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_url_locals_item(nested, out);
                }
            }
        }
        _ => {}
    }
}

fn collect_url_locals_expr(e: &Expr, out: &mut std::collections::HashSet<String>) {
    match e {
        Expr::Block(b) => collect_url_locals(&b.block, out),
        Expr::Unsafe(u) => collect_url_locals(&u.block, out),
        Expr::Loop(l) => collect_url_locals(&l.body, out),
        Expr::While(w) => {
            collect_url_locals_expr(&w.cond, out);
            collect_url_locals(&w.body, out);
        }
        Expr::If(i) => {
            collect_url_locals_expr(&i.cond, out);
            collect_url_locals(&i.then_branch, out);
            if let Some((_, else_e)) = &i.else_branch {
                collect_url_locals_expr(else_e, out);
            }
        }
        Expr::Match(m) => {
            for arm in &m.arms {
                collect_url_locals_expr(&arm.body, out);
            }
        }
        _ => {}
    }
}

/// A `format!(..)` call whose token text carries a `{...}` placeholder
/// (something dynamic went in). Token-text check only — never a descent.
fn is_interpolated_format(e: &Expr) -> bool {
    let Expr::Macro(mac) = e else {
        return false;
    };
    if !mac.mac.path.is_ident("format") {
        return false;
    }
    let text = mac.mac.tokens.to_string();
    // A `{` whose next char is not `}` — the empty `{}` placeholder has a
    // non-`}` char after `{` in the token stream too; `{{`/`}}` escapes
    // are the only false-negative risk (an escaped-brace-only URL is not
    // a dynamic URL — correctly silent).
    let bytes = text.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'{' && bytes.get(i + 1).is_some_and(|n| *n != b'{') {
            return true;
        }
    }
    false
}

fn scan_sinks(
    source: &str,
    block: &syn::Block,
    url_params: &std::collections::HashSet<String>,
    url_locals: &std::collections::HashSet<String>,
    findings: &mut Vec<EgressUrlFinding>,
) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_sinks_expr(source, &init.expr, url_params, url_locals, findings);
                }
            }
            Stmt::Item(item) => scan_sinks_item(source, item, url_params, url_locals, findings),
            Stmt::Expr(e, _) => scan_sinks_expr(source, e, url_params, url_locals, findings),
            Stmt::Macro(_) => {}
        }
    }
}

fn scan_sinks_item(
    source: &str,
    item: &Item,
    url_params: &std::collections::HashSet<String>,
    url_locals: &std::collections::HashSet<String>,
    findings: &mut Vec<EgressUrlFinding>,
) {
    match item {
        Item::Fn(f) => scan_sinks(source, &f.block, url_params, url_locals, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_sinks(source, &m.block, url_params, url_locals, findings);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    scan_sinks_item(source, nested, url_params, url_locals, findings);
                }
            }
        }
        _ => {}
    }
}

fn scan_sinks_expr(
    source: &str,
    e: &Expr,
    url_params: &std::collections::HashSet<String>,
    url_locals: &std::collections::HashSet<String>,
    findings: &mut Vec<EgressUrlFinding>,
) {
    if let Expr::Call(c) = e {
        if let Expr::Path(p) = c.func.as_ref() {
            if p.qself.is_none() && !p.path.segments.is_empty() {
                let first = p.path.segments[0].ident.to_string();
                if EGRESS_CRATES.contains(&first.as_str()) {
                    for a in &c.args {
                        if let Some(origin) = url_origin(a, url_params, url_locals) {
                            let span = e.span().byte_range();
                            if span.end <= source.len() {
                                findings.push(EgressUrlFinding {
                                    sink_text: source[span.clone()].to_string(),
                                    sink_span: span,
                                    url_origin: origin,
                                    advice: ADVICE,
                                });
                            }
                            break; // one report per sink call
                        }
                    }
                }
            }
        }
    }
    walk_children(e, &mut |child| {
        scan_sinks_expr(source, child, url_params, url_locals, findings);
    });
}

/// The URL origin of a sink argument: an inline interpolated `format!`,
/// a URL-vocabulary parameter, or a tracked local (through refs/parens).
fn url_origin(
    a: &Expr,
    url_params: &std::collections::HashSet<String>,
    url_locals: &std::collections::HashSet<String>,
) -> Option<String> {
    if is_interpolated_format(a) {
        return Some(String::from("format!"));
    }
    match strip_refs(a) {
        Expr::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 => {
            let id = p.path.segments[0].ident.to_string();
            if url_params.contains(&id) {
                Some(id)
            } else {
                url_locals.contains(&id).then_some(id)
            }
        }
        _ => None,
    }
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

    fn findings(src: &str) -> Vec<super::EgressUrlFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 258 gate — the corpus `buggy_code` MUST fire.
    #[test]
    fn batch258_interpolated_url_to_ureq_fires() {
        let src = "pub fn fetch_report(base: &str, probe: &str) -> Result<String, String> {\n    \
             let url = format!(\"https://{base}/report/{probe}\");\n    \
             let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;\n    \
             resp.into_string().map_err(|e| e.to_string())\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert!(
            f[0].sink_text.starts_with("ureq::get"),
            "{:?}",
            f[0].sink_text
        );
        assert_eq!(f[0].url_origin, "url");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// A URL-vocabulary PARAMETER reaching the sink fires directly.
    #[test]
    fn url_param_reaching_sink_fires() {
        let src = "pub fn fetch(endpoint: &str) -> Result<String, String> {\n    \
             let resp = reqwest::blocking::get(endpoint).map_err(|e| e.to_string())?;\n    \
             resp.text().map_err(|e| e.to_string())\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "param origin fires: {f:?}");
        assert_eq!(f[0].url_origin, "endpoint");
    }

    /// The allowlist-guarded (corpus fixed) form stays silent.
    #[test]
    fn allowlist_guard_suppresses() {
        let src = "const ALLOWED_HOSTS: [&str; 1] = [\"reports.internal\"];\n\n\
             pub fn fetch_report(base: &str, probe: &str) -> Result<String, String> {\n    \
             if !ALLOWED_HOSTS.contains(&base) {\n        \
             return Err(String::from(\"host not allowed\"));\n    \
             }\n    \
             let url = format!(\"https://{base}/report/{probe}\");\n    \
             let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;\n    \
             resp.into_string().map_err(|e| e.to_string())\n}\n";
        assert!(findings(src).is_empty(), "guarded fn must be silent");
    }

    /// A string-literal URL never fires (trusted by construction).
    #[test]
    fn literal_url_silent() {
        let src = "pub fn ping() -> Result<String, String> {\n    \
             let resp = ureq::get(\"https://status.internal/ping\").call().map_err(|e| e.to_string())?;\n    \
             resp.into_string().map_err(|e| e.to_string())\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A non-URL parameter reaching the sink is silent (the vocabulary is
    /// the honest shape).
    #[test]
    fn non_url_param_silent() {
        let src = "pub fn fetch(payload: &str) -> Result<String, String> {\n    \
             let resp = ureq::get(payload).call().map_err(|e| e.to_string())?;\n    \
             resp.into_string().map_err(|e| e.to_string())\n}\n";
        assert!(findings(src).is_empty(), "payload is not URL vocabulary");
    }

    /// A static `format!` (no placeholder) is silent — nothing dynamic
    /// went in.
    #[test]
    fn static_format_silent() {
        let src = "pub fn ping() -> Result<String, String> {\n    \
             let url = String::from(\"https://status.internal/ping\");\n    \
             let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;\n    \
             resp.into_string().map_err(|e| e.to_string())\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Client-binding method sinks are an honest NO (untypeable
    /// parse-only).
    #[test]
    fn client_method_sink_undetected() {
        let src = "pub fn fetch_client(client: &ureq::Agent, path: &str) -> Result<String, String> {\n    \
             let url = format!(\"https://internal{path}\");\n    \
             let resp = client.get(&url).call().map_err(|e| e.to_string())?;\n    \
             resp.into_string().map_err(|e| e.to_string())\n}\n";
        assert!(findings(src).is_empty(), "method sinks are an honest NO");
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn t(base: &str) {\n        \
             let url = format!(\"https://{base}/x\");\n        \
             let _ = ureq::get(&url).call();\n    \
             }\n}\n";
        assert!(findings(src).is_empty());
    }
}

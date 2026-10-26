//! `cors-any-origin-with-credentials` — SUGGEST-ONLY detector
//! (Batch 291, the danoszz/penthera distill; Research 329 / Plan 295).
//!
//! A permissive-origin CORS policy coupled with credentialed requests:
//! `allow_origin(Any)` / `AllowOrigin::any()` / `CorsLayer::permissive()`
//! chained with `allow_credentials(true)` — or the one-call form
//! `CorsLayer::very_permissive()`, which is credentials PLUS the preflight
//! origin reflected back as allowed (tower-http 0.7 docs verbatim: the
//! `origin: true` + `credentials: true` shape). The wildcard+credentials
//! pair is protocol-inert for credentialed reads (browsers refuse
//! credentials against a wildcard `Access-Control-Allow-Origin`), so the
//! declared grant is dead misconfiguration — and the common naive repair
//! (switch to `very_permissive`) ACTIVATES the leak: every origin's
//! credentialed cross-origin requests pass.
//!
//! ## Why suggest-only (never a fix)
//!
//! The legitimate origins are deployment policy (which origins call this
//! API is the operator's answer); a mechanical rewrite cannot invent them.
//! This module ships a parse-only DETECTION walk only: NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm — the suggestion surfaces through
//! retrieval (`--suggest` / the corpus entry) and review.
//!
//! ## The bounded form
//!
//! ```text
//! CorsLayer::new()
//!     .allow_origin(AllowOrigin::list(allowed.parse::<HeaderValue>()))
//!     .allow_credentials(true)
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Permissive-origin vocabulary, parse-provable only**: the method
//!   `allow_origin` called with a path argument whose last segment is
//!   `Any` (the `use tower_http::cors::Any` import form included);
//!   `AllowOrigin::any()` / `AllowOrigin::permissive()` /
//!   `AllowOrigin::mirror_request()`; `CorsLayer::permissive()`. List /
//!   predicate / dynamic origin forms never fire.
//! - **Credentials marker**: the method `allow_credentials` called with a
//!   literal `true` argument.
//! - **One-call combo fires unconditionally**: `very_permissive()` — the
//!   credentials + reflection coupling is its by-construction content.
//! - **Co-occurrence scope**: one fn body (the plain-fn + impl-fn walk;
//!   `#[cfg(test)]` mods skipped — the house discipline). `permissive()`
//!   alone is silent (tower-http sets NO credentials on it); the
//!   permissive-origin marker needs the credentials marker in the same fn.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - `allow_origin(Any)` WITHOUT a credentials marker stays silent — bare
//!   wildcard CORS is a policy smell, not this graded class.
//! - Cross-fn builder assembly (built in a helper, hardened in the caller)
//!   is not connected parse-only.
//! - The vocabulary proves a CONFIGURATION CLASS, not a leak — the
//!   `any()+credentials` form is inert per the CORS protocol and review
//!   decides; suggest-only is the blast-radius control.
//! - Batch 291 / Plan 295 / Research 329 (danoszz/penthera @ `07421f9`,
//!   MIT — cited as class provenance; the fixture is an original
//!   synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::{Expr, File, Item};

/// One detected permissive-origins-with-credentials CORS site.
#[derive(Debug, Clone)]
pub struct CorsAnyCredsFinding {
    /// Byte range of the offending marker (the credentials call, or the
    /// one-call `very_permissive()`).
    pub site_span: Range<usize>,
    /// The marker call's source text.
    pub site_text: String,
    /// Which marker fired (the combo kind).
    pub marker: &'static str,
    /// The advice line (allowlist or drop credentials + the class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "couple permissive CORS origins with credentials: \
list the exact origins (`AllowOrigin::list` / the origins-array form) beside \
`allow_credentials(true)`, or drop credentials; `CorsLayer::very_permissive()` \
reflects any origin AND allows credentials (every origin's credentialed \
cross-origin requests pass; Batch 291, suggest-only)";

/// Detect permissive-origin + credentialed CORS configurations
/// (suggest-only; no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<CorsAnyCredsFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<CorsAnyCredsFinding>) {
    match item {
        Item::Fn(f) => scan_shaped(source, &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_shaped(source, &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production CORS configuration.
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

fn scan_shaped(source: &str, block: &syn::Block, findings: &mut Vec<CorsAnyCredsFinding>) {
    let mut walk = FnScan::default();
    scan_stmts(&block.stmts, (source, &mut walk));

    // One-call combo: fires alone.
    if let Some(span) = walk.very_permissive_span {
        findings.push(CorsAnyCredsFinding {
            site_text: source[span.clone()].to_string(),
            site_span: span,
            marker: "very_permissive",
            advice: ADVICE,
        });
    }
    // The combo: permissive-origin marker + explicit credentials.
    if walk.permissive_origin {
        if let Some(span) = walk.credentials_span {
            findings.push(CorsAnyCredsFinding {
                site_text: source[span.clone()].to_string(),
                site_span: span,
                marker: "permissive_origin_plus_credentials",
                advice: ADVICE,
            });
        }
    }
}

#[derive(Default)]
struct FnScan {
    permissive_origin: bool,
    credentials_span: Option<Range<usize>>,
    very_permissive_span: Option<Range<usize>>,
}
fn scan_stmts(stmts: &[syn::Stmt], ctx: (&str, &mut FnScan)) {
    let (source, walk) = ctx;
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    walk_expr(&init.expr, source, walk);
                }
            }
            syn::Stmt::Item(item) => collect_item_exprs(item, source, walk),
            syn::Stmt::Expr(e, _) => walk_expr(e, source, walk),
            syn::Stmt::Macro(_) => {}
        }
    }
}

fn collect_item_exprs(item: &Item, source: &str, walk: &mut FnScan) {
    match item {
        Item::Fn(f) => scan_stmts(&f.block.stmts, (source, walk)),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_stmts(&m.block.stmts, (source, walk));
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_item_exprs(nested, source, walk);
                }
            }
        }
        _ => {}
    }
}

fn walk_expr(e: &Expr, source: &str, walk: &mut FnScan) {
    match e {
        Expr::MethodCall(m) => {
            let name = m.method.to_string();
            if name == "very_permissive" {
                if walk.very_permissive_span.is_none() {
                    walk.very_permissive_span = span_of(e, source);
                }
            } else if name == "allow_credentials" && is_true_arg(m.args.first()) {
                if walk.credentials_span.is_none() {
                    walk.credentials_span = span_of(e, source);
                }
            } else if name == "allow_origin" && m.args.iter().any(is_permissive_origin_arg) {
                walk.permissive_origin = true;
            } else if name == "permissive" {
                // `CorsLayer::permissive()` sets NO credentials (tower-http
                // 0.7 docs) — it is a permissive-origin marker only.
                walk.permissive_origin = true;
            }
            walk_expr(&m.receiver, source, walk);
            for a in &m.args {
                walk_expr(a, source, walk);
            }
        }
        Expr::Call(c) => {
            if let Expr::Path(p) = c.func.as_ref() {
                let segs: Vec<String> = p
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                // Associated-fn forms: AllowOrigin::any()/permissive()/
                // mirror_request() are permissive-origin markers;
                // CorsLayer::very_permissive() is the one-call combo;
                // CorsLayer::permissive() is a permissive-origin marker
                // (tower-http 0.7: it sets NO credentials). All parse as
                // Expr::Call with a path func (no receiver), unlike the
                // builder-chain method calls.
                if segs.len() >= 2 {
                    let type_seg = &segs[segs.len() - 2];
                    let m = segs[segs.len() - 1].as_str();
                    if type_seg == "AllowOrigin"
                        && (m == "any" || m == "permissive" || m == "mirror_request")
                    {
                        walk.permissive_origin = true;
                    } else if type_seg == "CorsLayer" {
                        if m == "very_permissive" {
                            if walk.very_permissive_span.is_none() {
                                walk.very_permissive_span = span_of(e, source);
                            }
                        } else if m == "permissive" {
                            walk.permissive_origin = true;
                        }
                    }
                }
            }
            walk_expr(&c.func, source, walk);
            for a in &c.args {
                walk_expr(a, source, walk);
            }
        }
        _ => scan_child_exprs(e, source, walk),
    }
}

fn scan_child_exprs(e: &Expr, source: &str, walk: &mut FnScan) {
    // Control-flow shapes that can carry the builder chain (macro tokens
    // are never descended into — the house recall limit).
    match e {
        Expr::If(i) => {
            walk_expr(&i.cond, source, walk);
            scan_stmts(&i.then_branch.stmts, (source, walk));
            if let Some((_, else_e)) = &i.else_branch {
                walk_expr(else_e, source, walk);
            }
        }
        Expr::Match(m) => {
            walk_expr(&m.expr, source, walk);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    walk_expr(guard, source, walk);
                }
                walk_expr(&arm.body, source, walk);
            }
        }
        Expr::Block(b) => scan_stmts(&b.block.stmts, (source, walk)),
        Expr::Unsafe(u) => scan_stmts(&u.block.stmts, (source, walk)),
        Expr::Loop(l) => scan_stmts(&l.body.stmts, (source, walk)),
        Expr::While(w) => {
            walk_expr(&w.cond, source, walk);
            scan_stmts(&w.body.stmts, (source, walk));
        }
        Expr::ForLoop(fl) => {
            walk_expr(&fl.expr, source, walk);
            scan_stmts(&fl.body.stmts, (source, walk));
        }
        _ => {}
    }
}

/// The `Any` origin token in an `allow_origin(..)` argument position (the
/// bare-import form; a qualified `cors::Any` path also ends in `Any`).
fn is_permissive_origin_arg(a: &Expr) -> bool {
    let Expr::Path(p) = a else {
        return false;
    };
    p.qself.is_none() && p.path.segments.last().is_some_and(|s| s.ident == "Any")
}

/// A literal `true` argument (`allow_credentials(true)`).
fn is_true_arg(a: Option<&Expr>) -> bool {
    let Some(Expr::Lit(l)) = a else {
        return false;
    };
    match &l.lit {
        syn::Lit::Bool(b) => b.value(),
        _ => false,
    }
}

fn span_of(e: &Expr, source: &str) -> Option<Range<usize>> {
    let span = syn::spanned::Spanned::span(e).byte_range();
    (span.end <= source.len()).then_some(span)
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::CorsAnyCredsFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 291 gate — the corpus `buggy_code` MUST fire.
    #[test]
    fn batch291_any_plus_credentials_fires() {
        let src = "use tower_http::cors::{Any, CorsLayer};\n\n\
             pub fn permissive_cors() -> tower_http::cors::CorsLayer {\n    \
             CorsLayer::new()\n        .allow_origin(Any)\n        \
             .allow_credentials(true)\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].marker, "permissive_origin_plus_credentials");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The one-call combo fires unconditionally.
    #[test]
    fn very_permissive_fires_alone() {
        let src = "pub fn open_api() -> tower_http::cors::CorsLayer {\n    \
             tower_http::cors::CorsLayer::very_permissive()\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "very_permissive is the one-call combo: {f:?}");
        assert_eq!(f[0].marker, "very_permissive");
    }

    /// The fully-qualified `AllowOrigin::any()` form fires with
    /// credentials.
    #[test]
    fn allow_origin_any_path_form_fires() {
        let src = "pub fn layer() -> tower_http::cors::CorsLayer {\n    \
             tower_http::cors::CorsLayer::new()\n        \
             .allow_origin(tower_http::cors::AllowOrigin::any())\n        \
             .allow_credentials(true)\n}\n";
        assert_eq!(findings(src).len(), 1);
    }

    /// `allow_origin(Any)` WITHOUT credentials stays silent (bare
    /// wildcard is not this graded class).
    #[test]
    fn wildcard_without_credentials_silent() {
        let src = "use tower_http::cors::{Any, CorsLayer};\n\n\
             pub fn public_assets() -> CorsLayer {\n    \
             CorsLayer::new().allow_origin(Any).allow_methods(Any)\n}\n";
        assert!(findings(src).is_empty(), "no credentials marker: silent");
    }

    /// Plain `permissive()` alone is silent (tower-http sets NO
    /// credentials on it).
    #[test]
    fn permissive_alone_silent() {
        let src = "pub fn lax_api() -> tower_http::cors::CorsLayer {\n    \
             tower_http::cors::CorsLayer::permissive()\n}\n";
        assert!(findings(src).is_empty());
    }

    /// `allow_credentials(false)` is not a credentials marker.
    #[test]
    fn credentials_false_silent() {
        let src = "use tower_http::cors::{Any, CorsLayer};\n\n\
             pub fn anon_only() -> CorsLayer {\n    \
             CorsLayer::new().allow_origin(Any).allow_credentials(false)\n}\n";
        assert!(findings(src).is_empty());
    }

    /// The allowlisted (corpus fixed) form stays silent.
    #[test]
    fn allowlist_form_silent() {
        let src = "use tower_http::cors::{AllowOrigin, CorsLayer};\n\n\
             pub fn allowlisted_cors() -> CorsLayer {\n    \
             CorsLayer::new()\n        \
             .allow_origin(AllowOrigin::list(origins()))\n        \
             .allow_credentials(true)\n}\n\n\
             fn origins() -> impl Iterator<Item = http::HeaderValue> {\n    \
             std::iter::empty()\n}\n";
        assert!(findings(src).is_empty(), "list form never fires");
    }

    /// An unrelated `Any` value (`std::any::Any`-shaped) never fires.
    #[test]
    fn unrelated_any_silent() {
        let src = "pub fn store(v: Box<dyn std::any::Any>) -> Box<dyn std::any::Any> {\n    \
             v\n}\n";
        assert!(findings(src).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn t() {\n        \
             let _ = tower_http::cors::CorsLayer::very_permissive();\n    \
             }\n}\n";
        assert!(findings(src).is_empty());
    }
}

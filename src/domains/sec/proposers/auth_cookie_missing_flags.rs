//! `auth-cookie-without-secure-flags` — SUGGEST-ONLY detector
//! (Batch 291, the danoszz/penthera distill; Research 329 / Plan 295).
//!
//! An auth/session-named cookie built WITHOUT the Secure/HttpOnly
//! hardening: the cookie crate's `Cookie::build(..)` / `Cookie::new(..)`
//! with a literal name carrying the session vocabulary
//! (session/sid/token/auth/jwt — penthera's `/sess|token|auth|jwt|sid/i`
//! severity gate) and no `.secure(..)` / `.http_only(..)` marker anywhere
//! in the fn. The session value ships JavaScript-readable over plain
//! http: theft-by-XSS and plaintext-interception, respectively.
//!
//! ## Why suggest-only (never a fix)
//!
//! Cookies that must be JS-readable (an SPA token handoff) or cross-site
//! are deliberate choices; a mechanical rewrite cannot know which. This
//! module ships a parse-only DETECTION walk only: NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm — the suggestion surfaces through
//! retrieval (`--suggest` / the corpus entry) and review.
//!
//! ## The bounded form
//!
//! ```text
//! Cookie::build(("sid", value.to_owned()))
//!     .path("/")
//!     .http_only(true)
//!     .secure(true)
//!     .same_site(cookie::SameSite::Lax)
//!     .build()
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Name gate (the FP control)**: the cookie name must be a string
//!   LITERAL carrying the vocabulary (direct arg, or the first element of
//!   the `("sid", value)` tuple form). Variable-named cookies are an
//!   honest NO; preference/tracking/consent names never fire.
//! - **Fn-wide flag suppression (the secret-file-0600 conservative
//!   rule)**: any `set_http_only` / `set_secure` / `.http_only(` /
//!   `.secure(` marker anywhere in the fn suppresses the whole fn — one
//!   hardened cookie hides a sibling, and an explicit `.secure(false)` is
//!   a written decision, never flagged.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped;
//!   macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - The vocabulary proves a NAME CLASS, not a session — an internal
//!   analytics cookie named `auth_trace` fires too, and review decides.
//! - Builder-split-across-locals with zero fn-level markers fires on the
//!   construction site (correct for the default-leak class); the
//!   fn-wide marker covers the `set_http_only`-after-construction forms.
//! - Framework extractors (`axum::http::header::SET_COOKIE` writes) are
//!   out of scope — the cookie-crate vocabulary is the parse-provable
//!   surface.
//! - Batch 291 / Plan 295 / Research 329 (danoszz/penthera @ `07421f9`,
//!   MIT — cited as class provenance; the fixture is an original
//!   synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item};

/// One detected unhardened auth/session cookie construction.
#[derive(Debug, Clone)]
pub struct AuthCookieFinding {
    /// Byte range of the `Cookie::build`/`Cookie::new` call.
    pub site_span: Range<usize>,
    /// The call's source text.
    pub site_text: String,
    /// The literal cookie name that tripped the vocabulary gate.
    pub cookie_name: String,
    /// The advice line (the builder chain + the class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "harden session cookies at construction: \
`.http_only(true)` (theft-by-XSS) + `.secure(true)` (plaintext \
interception) + `.same_site(SameSite::Lax)` (CSRF) on the builder chain — \
a cookie that must be JS-readable or cross-site is a deliberate choice \
review should see (Batch 291, suggest-only)";

/// The session-vocabulary tokens (substring scan, lowercase — the
/// penthera severity gate: `/sess|token|auth|jwt|sid/i`).
const AUTH_NAME_TOKENS: &[&str] = &["sess", "token", "auth", "jwt", "sid"];

/// The fn-wide flag markers (token-text, space-stripped): ANY of these
/// suppresses the whole fn (one hardened cookie hides a sibling — the
/// secret-file-create conservative rule).
const FLAG_MARKERS: &[&str] = &["set_http_only", "set_secure", ".http_only(", ".secure("];

/// Detect unhardened auth/session cookie constructions (suggest-only; no
/// edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<AuthCookieFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<AuthCookieFinding>) {
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
            // fixture habitat, not production cookie construction.
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

fn scan_fn(source: &str, block: &syn::Block, findings: &mut Vec<AuthCookieFinding>) {
    // Fn-wide flag gate (token text, space-stripped): one hardened cookie
    // (or an explicit `secure(false)`) hides a sibling — conservative.
    let fn_ts = quote::ToTokens::to_token_stream(block).to_string();
    let fn_text = fn_ts.replace(' ', "");
    if FLAG_MARKERS.iter().any(|m| fn_text.contains(m)) {
        return;
    }

    scan_stmts(source, &block.stmts, findings);
}

fn scan_stmts(source: &str, stmts: &[syn::Stmt], findings: &mut Vec<AuthCookieFinding>) {
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_expr(source, &init.expr, findings);
                }
            }
            syn::Stmt::Item(item) => collect_in_item(source, item, findings),
            syn::Stmt::Expr(e, _) => scan_expr(source, e, findings),
            syn::Stmt::Macro(_) => {}
        }
    }
}

fn scan_expr(source: &str, e: &Expr, findings: &mut Vec<AuthCookieFinding>) {
    match e {
        // A method chain: the construction site lives at the RECEIVER spine
        // root — report it ONCE at the innermost call, then walk only the
        // chain's arguments (the chain itself is one statement-level
        // expression; re-walking the receiver re-reports the same site).
        Expr::MethodCall(m) => {
            scan_expr(source, &m.receiver, findings);
            for a in &m.args {
                scan_expr(source, a, findings);
            }
        }
        Expr::Call(c) => {
            scan_call(c, source, findings);
            scan_expr(source, &c.func, findings);
            for a in &c.args {
                scan_expr(source, a, findings);
            }
        }
        Expr::If(i) => {
            scan_expr(source, &i.cond, findings);
            scan_stmts(source, &i.then_branch.stmts, findings);
            if let Some((_, else_e)) = &i.else_branch {
                scan_expr(source, else_e, findings);
            }
        }
        Expr::Match(m) => {
            scan_expr(source, &m.expr, findings);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    scan_expr(source, guard, findings);
                }
                scan_expr(source, &arm.body, findings);
            }
        }
        Expr::Block(b) => scan_stmts(source, &b.block.stmts, findings),
        Expr::Unsafe(u) => scan_stmts(source, &u.block.stmts, findings),
        Expr::Loop(l) => scan_stmts(source, &l.body.stmts, findings),
        Expr::While(w) => {
            scan_expr(source, &w.cond, findings);
            scan_stmts(source, &w.body.stmts, findings);
        }
        Expr::ForLoop(fl) => {
            scan_expr(source, &fl.expr, findings);
            scan_stmts(source, &fl.body.stmts, findings);
        }
        _ => {}
    }
}

/// The construction site: last path segments `Cookie::build` /
/// `Cookie::new`, with a literal auth-vocabulary name.
fn scan_call(c: &syn::ExprCall, source: &str, findings: &mut Vec<AuthCookieFinding>) {
    let Expr::Path(p) = c.func.as_ref() else {
        return;
    };
    if p.qself.is_some() || p.path.segments.len() < 2 {
        return;
    }
    let type_seg = p.path.segments[p.path.segments.len() - 2].ident.to_string();
    let ctor = p.path.segments[p.path.segments.len() - 1].ident.to_string();
    if type_seg != "Cookie" || (ctor != "build" && ctor != "new") {
        return;
    }
    let Some(name) = cookie_name_arg(c.args.first()) else {
        return; // variable-named: an honest NO
    };
    if !is_auth_name(&name) {
        return; // preference/tracking/consent: silent
    }
    let span = c.span().byte_range();
    if span.end > source.len() {
        return; // the parse came from another file (the egress guard)
    }
    findings.push(AuthCookieFinding {
        site_text: source[span.clone()].to_string(),
        site_span: span,
        cookie_name: name,
        advice: ADVICE,
    });
}

/// The literal cookie name: a string-literal first arg, or the first
/// element of the `("sid", value)` tuple form (cookie 0.18's
/// `Cookie::build(("name", value))`).
fn cookie_name_arg(a: Option<&Expr>) -> Option<String> {
    let first = a?;
    match first {
        Expr::Lit(l) => match &l.lit {
            syn::Lit::Str(s) => Some(s.value()),
            _ => None,
        },
        Expr::Tuple(t) => match t.elems.first() {
            Some(Expr::Lit(l)) => match &l.lit {
                syn::Lit::Str(s) => Some(s.value()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

/// The penthera severity vocabulary: `/sess|token|auth|jwt|sid/i` on the
/// literal name.
fn is_auth_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    AUTH_NAME_TOKENS.iter().any(|t| lower.contains(t))
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::AuthCookieFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 291 gate — the corpus `buggy_code` MUST fire.
    #[test]
    fn batch291_session_cookie_without_flags_fires() {
        let src = "use cookie::Cookie;\n\n\
             pub fn session_cookie(value: &str) -> Cookie<'static> {\n    \
             Cookie::build((\"sid\", value.to_owned()))\n        \
             .path(\"/\")\n        \
             .same_site(cookie::SameSite::Lax)\n        \
             .build()\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].cookie_name, "sid");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The `Cookie::new` direct-name form fires.
    #[test]
    fn cookie_new_form_fires() {
        let src = "pub fn token_cookie(v: String) -> cookie::Cookie<'static> {\n    \
             cookie::Cookie::new(\"auth_token\", v)\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].cookie_name, "auth_token");
    }

    /// A hardened fn (the corpus fixed form) stays silent — fn-wide
    /// marker suppression.
    #[test]
    fn hardened_fn_silent() {
        let src = "pub fn session_cookie(value: &str) -> Cookie<'static> {\n    \
             Cookie::build((\"sid\", value.to_owned()))\n        \
             .path(\"/\")\n        \
             .http_only(true)\n        \
             .secure(true)\n        \
             .same_site(cookie::SameSite::Lax)\n        \
             .build()\n}\n";
        assert!(findings(src).is_empty(), "flags present: silent");
    }

    /// An explicit `.secure(false)` is a written decision — silent
    /// (the fn-wide marker covers it).
    #[test]
    fn explicit_false_silent() {
        let src = "pub fn csrf_probe(v: &str) -> Cookie<'static> {\n    \
             Cookie::build((\"csrf\", v.to_owned()))\n        \
             .secure(false)\n        \
             .build()\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A non-auth name (preference/tracking) never fires — the
    /// vocabulary is the FP control.
    #[test]
    fn preference_cookie_silent() {
        let src = "pub fn theme(v: &str) -> Cookie<'static> {\n    \
             Cookie::new(\"theme\", v)\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A variable-named cookie is an honest NO.
    #[test]
    fn variable_name_silent() {
        let src = "pub fn named(name: &str, v: &str) -> Cookie<'static> {\n    \
             Cookie::new(name, v)\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A setter AFTER construction (builder split across statements)
    /// suppresses via the fn-wide marker.
    #[test]
    fn setter_after_construction_silent() {
        let src = "pub fn session(v: &str) -> Cookie<'static> {\n    \
             let mut c = Cookie::new(\"session\", v.to_owned());\n    \
             c.set_http_only(true);\n    \
             c\n}\n";
        assert!(findings(src).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn t() {\n        \
             let _ = Cookie::new(\"session\", \"x\");\n    \
             }\n}\n";
        assert!(findings(src).is_empty());
    }
}

//! `open-redirect-unvalidated-target` — SUGGEST-ONLY detector
//! (Batch 351, the 0sec-labs/foxguard distill; Research 394 / Plan 361).
//! The open-redirect family seed — zero prior corpus owners.
//!
//! A redirect target built from caller-controlled input —
//! `Redirect::to(&user_url)` / an axum-actix redirect whose argument is
//! a non-constant string — lets a crafted link bounce a user through
//! YOUR origin to an attacker-chosen destination (phishing that shows
//! a legitimate URL in the bar, OAuth-return-token theft; CWE-601).
//!
//! ## The source anchor (0sec-labs/foxguard @ `fbeab16`)
//!
//! `src/rules/python.rs` `NoOpenRedirect`, verbatim at the pin: the
//! SAFE half is explicit — "Framework helpers that build a safe,
//! server-controlled URL from a route/view name. A redirect to their
//! result is not user-controlled" (`url_for` / `reverse` /
//! `get_absolute_url` family) — plus `collect_constant_bindings`:
//! "Identifiers proven to hold only constant literals are not dynamic."
//! The Rust fix-space transcribes: relative-only normalization, the
//! route-name builder, or an explicit external-host allowlist.
//!
//! ## Why suggest-only (never a fix)
//!
//! The allowlist's CONTENT is the deployment's route/host policy and
//! the redirect API is framework-specific. NO `FIXABLE_SEC` membership,
//! NO `collect_edits` arm.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Redirect vocabulary is fn-scope (`redirect` / `Redirect::to` /
//!   `HttpResponse::Found` shapes); a redirect assembled across helpers
//!   is unconnected parse-only.
//! - A deliberate external redirect (an OAuth provider's hosted pages,
//!   a documented partner URL) fires and review decides — the rule
//!   flags the pattern class, never a policy decision.
//! - Const-proven operands are silent (the source's constant-binding
//!   law): `Redirect::to("/dashboard")` and a `const` path never fire.
//! - Detector: `proposers::open_redirect_gate`. Fixture:
//!   `fixtures/b351_fixtures.rs` F5. Batch 351 / Plan 361 / Research 394
//!   (0sec-labs/foxguard @ `fbeab16`, MIT OR Apache-2.0 — cited as class
//!   provenance; the fixture is an original synthesis, standalone-compiled
//!   AND executed before entry).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item};

/// One detected unvalidated redirect target.
#[derive(Debug, Clone)]
pub struct RedirectFinding {
    /// Byte range of the redirect-call expression.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the relative-only + route-builder law).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a redirect target built from caller-controlled \
input bounces users through your origin to an attacker-chosen URL \
(CWE-601 — phishing with a legitimate referrer, OAuth token theft). \
Normalize to relative paths only (refuse absolute and \
protocol-relative targets), redirect by route name \
(`url_for`/`reverse`-family builders are server-controlled), or \
allowlist the explicit external hosts (Batch 351, suggest-only)";

/// The redirect vocabulary (the class-shape gate).
const REDIRECT_TOKENS: &[&str] = &[
    "Redirect::to",
    "redirect(",
    "HttpResponse::Found",
    "SeeOther",
    "Temporary",
];

/// The source's safe half: the route-name builder family (server-
/// controlled by construction).
const ROUTE_BUILDERS: &[&str] = &[
    "url_for",
    "url_for_external",
    "reverse",
    "reverse_lazy",
    "get_absolute_url",
];

/// Detect unvalidated redirect targets (suggest-only; no edits are
/// produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<RedirectFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<RedirectFinding>) {
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
            // fixture habitat, not production redirect edges.
            if m.attrs.iter().any(|a| {
                a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test")
            }) {
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

fn scan_fn(source: &str, block: &syn::Block, findings: &mut Vec<RedirectFinding>) {
    let fn_ts = quote::ToTokens::to_token_stream(block).to_string();
    // Token streams render with spaces ("Redirect :: to (..)") — compact
    // before the vocabulary checks.
    let compact: String = fn_ts.chars().filter(|c| !c.is_whitespace()).collect();
    // Class-shape gate: redirect vocabulary in the fn.
    if !REDIRECT_TOKENS.iter().any(|t| compact.contains(t)) {
        return;
    }
    // Suppression A (the source's safe half): a route-name builder in
    // the fn — the target is server-controlled.
    if ROUTE_BUILDERS.iter().any(|t| compact.contains(t)) {
        return;
    }
    // Suppression B (the source's constant-binding law): a const
    // declaration in the fn feeding the redirect is the static posture
    // (checked on the RAW stream — "const " with its space — so the
    // compacted "constructor" cannot false-match).
    if fn_ts.contains("const ") {
        return;
    }
    scan_stmts(&block.stmts, source, findings);
}

fn scan_stmts(stmts: &[syn::Stmt], source: &str, findings: &mut Vec<RedirectFinding>) {
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_expr(&init.expr, source, findings);
                }
            }
            syn::Stmt::Item(Item::Fn(f)) => scan_stmts(&f.block.stmts, source, findings),
            // Block descent FIRST (issue 227 D1): a redirect site inside
            // an if/match/block body is as reachable as a bare statement.
            syn::Stmt::Expr(inner, _) if block_shaped(inner) => {
                descend_block_expr(inner, source, findings);
            }
            syn::Stmt::Expr(e, _) => scan_expr(e, source, findings),
            _ => {}
        }
    }
}

/// The block-shaped expressions the descent visits.
fn block_shaped(e: &Expr) -> bool {
    matches!(
        e,
        Expr::If(_) | Expr::Match(_) | Expr::Block(_) | Expr::Unsafe(_) | Expr::Return(_)
    )
}

/// Descend a block-shaped expression's bodies (the redirect scan applies
/// to the same expressions a bare-expression walk would see).
fn descend_block_expr(e: &Expr, source: &str, findings: &mut Vec<RedirectFinding>) {
    match e {
        Expr::If(eif) => {
            scan_stmts(&eif.then_branch.stmts, source, findings);
            if let Some((_, else_expr)) = &eif.else_branch {
                scan_expr(else_expr, source, findings);
            }
        }
        Expr::Match(m) => {
            for arm in &m.arms {
                scan_expr(&arm.body, source, findings);
            }
        }
        Expr::Block(b) => scan_stmts(&b.block.stmts, source, findings),
        Expr::Unsafe(u) => scan_stmts(&u.block.stmts, source, findings),
        Expr::Return(r) => {
            if let Some(ret) = &r.expr {
                scan_expr(ret, source, findings);
            }
        }
        _ => {}
    }
}

fn scan_expr(e: &Expr, source: &str, findings: &mut Vec<RedirectFinding>) {
    match e {
        Expr::Call(c) => {
            // The associated-function form: `Redirect::to(target)` — the
            // func IS a path whose last segment names the redirect
            // constructor.
            if let Expr::Path(p) = &*c.func {
                let last = p
                    .path
                    .segments
                    .last()
                    .map_or(String::new(), |s| s.ident.to_string());
                let path_ts = quote::ToTokens::to_token_stream(&p.path).to_string();
                let vocab = path_ts.contains("Redirect") || path_ts.contains("HttpResponse");
                let ctor = last == "to" || last == "redirect" || last == "temporary";
                if vocab && ctor && !args_all_literal(&c.args) {
                    let byte_range = e.span().byte_range();
                    findings.push(RedirectFinding {
                        site_span: byte_range.clone(),
                        site_text: source.get(byte_range).unwrap_or("").to_string(),
                        advice: ADVICE,
                    });
                    return;
                }
            }
            // Recurse (a call inside a builder chain, argument positions).
            scan_expr(&c.func, source, findings);
            for a in &c.args {
                scan_expr(a, source, findings);
            }
        }
        Expr::MethodCall(mc) => {
            let method = mc.method.to_string();
            if (method == "redirect" || method == "temporary")
                && redirect_vocab_present(mc)
                && !args_all_literal(&mc.args)
            {
                let byte_range = e.span().byte_range();
                findings.push(RedirectFinding {
                    site_span: byte_range.clone(),
                    site_text: source.get(byte_range).unwrap_or("").to_string(),
                    advice: ADVICE,
                });
                return;
            }
            // Recurse into the receiver (a builder chain around the
            // redirect call: `Redirect::to(t).finish()`).
            scan_expr(&mc.receiver, source, findings);
            for a in &mc.args {
                scan_expr(a, source, findings);
            }
        }
        _ => {}
    }
}

fn redirect_vocab_present(mc: &syn::ExprMethodCall) -> bool {
    let recv_ts = quote::ToTokens::to_token_stream(&mc.receiver).to_string();
    let method_ts = quote::ToTokens::to_token_stream(&mc.method).to_string();
    recv_ts.contains("Redirect") || method_ts == "redirect" || recv_ts.contains("HttpResponse")
}

/// The constant-binding law: an all-literal argument list is the static
/// posture.
fn args_all_literal(args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>) -> bool {
    !args.is_empty()
        && args
            .iter()
            .all(|a| matches!(a, Expr::Lit(_) | Expr::Const(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(src_text: &str) -> Vec<RedirectFinding> {
        let file = syn::parse_file(src_text).expect("parses");
        detect(src_text, &file)
    }

    /// Fires on its own corpus buggy fixture (the self-fire law).
    #[test]
    fn fires_on_own_buggy_fixture() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "open-redirect-unvalidated-target")
            .expect("rule present");
        assert!(
            !findings(&rule.buggy_code).is_empty(),
            "the proposer must fire on its own buggy fixture"
        );
    }

    /// Batch 351 gate — the corpus `fixed_code` MUST stay silent (the
    /// fix normalizes relative-only + route-name builders).
    #[test]
    fn corpus_fixed_fixture_is_silent() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "open-redirect-unvalidated-target")
            .expect("rule present");
        assert!(
            findings(&rule.fixed_code).is_empty(),
            "the fixed fixture must stay silent"
        );
    }

    /// The dynamic target fires; the literal target is silent (the
    /// constant-binding law).
    #[test]
    fn dynamic_fires_literal_silent() {
        let dynamic = "pub fn bounce(target: &str) -> Response {\n    \
             Redirect::to(target).finish()\n}\n";
        assert_eq!(findings(dynamic).len(), 1, "the dynamic target fires");
        let literal = "pub fn home() -> Response {\n    \
             Redirect::to(\"/dashboard\").finish()\n}\n";
        assert!(
            findings(literal).is_empty(),
            "a literal target is the static posture"
        );
    }

    /// The route-name builder suppresses (the source's safe half).
    #[test]
    fn route_builder_suppresses() {
        let src_text = "pub fn bounce_to_profile(user: &User) -> Response {\n    \
             let target = url_for(\"profile\", user.id);\n    \
             Redirect::to(&target).finish()\n}\n";
        assert!(findings(src_text).is_empty());
    }

    /// The class-shape gate: no redirect vocabulary → silent.
    #[test]
    fn no_redirect_vocab_silent() {
        let src_text = "pub fn build_link(target: &str) -> String {\n    \
             format!(\"https://cdn.example/{target}\")\n}\n";
        assert!(findings(src_text).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src_text = "#[cfg(test)]\nmod tests {\n    \
             fn t() -> Response {\n        \
             Redirect::to(\"/x\").finish()\n    }\n}\n";
        assert!(findings(src_text).is_empty());
    }
}

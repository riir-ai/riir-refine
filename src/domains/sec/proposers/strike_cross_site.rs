//! `abuse-strike-counts-cross-site-requests` — SUGGEST-ONLY detector
//! (Batch 292, the hevnsnt/`CVEngine` distill; Research 330 / Plan 296).
//!
//! An abuse/strike counter incremented on EVERY rejected request
//! regardless of the browser's site label: a hostile page can make a
//! victim's browser send attack-shaped requests (an image tag, an
//! auto-submitted form), so counting those rejections bans the VICTIM —
//! denial-of-service via the defense itself. `CVEngine`'s guard docstring
//! is the rule's decision content: requests a browser marks cross-site
//! are rejected but never counted, so a hostile page cannot get a
//! visitor banned.
//!
//! ## Why suggest-only (never a fix)
//!
//! Which labels to honor (sec-fetch-site vs Origin/Referer fallback) and
//! the trust ordering are deployment policy. This module ships a
//! parse-only DETECTION walk only: NO `FIXABLE_SEC` membership, NO
//! `collect_edits` arm — the suggestion surfaces through retrieval
//! (`--suggest` / the corpus entry) and review.
//!
//! ## The bounded form
//!
//! ```text
//! if attack_shaped(body) {
//!     if site_label != Some("same-site") {
//!         return Verdict::Rejected; // reject, never count
//!     }
//!     self.strikes += 1;
//!     return Verdict::Rejected;
//! }
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Strike vocabulary (the FP control)**: the fn's token text must
//!   carry strike/abuse/penalty increment vocabulary (strike / abuse /
//!   penalty).
//! - **Rejection marker**: a rejection shape in the same fn (`400` /
//!   `403` / `BAD_REQUEST` / `FORBIDDEN` / `reject` / `Rejected`).
//! - **Site-label suppression (fn-wide)**: any site-label token
//!   (`sec_fetch` / `sec-fetch-site` / `same-site` / `same_site` /
//!   `cross-site` / `cross_site` / `origin_header` / `referer`) means the
//!   guard already classifies — silent.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped;
//!   macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - Both gates are fn-level: counting split across fns is unconnected
//!   parse-only.
//! - A strike counter with no rejection path (game score, retry budget)
//!   stays silent by construction.
//! - The vocabulary proves a CLASS, not a verdict: an internal job
//!   limiter with reject-shaped errors fires too, and review decides.
//! - Batch 292 / Plan 296 / Research 330 (hevnsnt/`CVEngine` @ `9dd9c441`,
//!   GPL-3.0 — cited as class provenance; the fixture is an original
//!   synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item};

/// One detected uncounted-cross-site strike site.
#[derive(Debug, Clone)]
pub struct StrikeCrossSiteFinding {
    /// Byte range of the strike-vocabulary increment expression.
    pub site_span: Range<usize>,
    /// The increment expression's source text.
    pub site_text: String,
    /// The advice line (the site-label gate + the class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "gate the strike increment on the browser's site \
label: a hostile page can forge attack-shaped requests from a victim's browser, \
so counting every rejection bans the victim — reject cross-site/same-site-labeled \
requests (`sec-fetch-site`) WITHOUT counting them, count only first-party \
traffic (Batch 292, suggest-only)";

/// The strike/abuse vocabulary (token-text substrings, lowercase).
const STRIKE_TOKENS: &[&str] = &["strike", "abuse", "penalty"];

/// The rejection markers (token-text substrings, lowercase).
const REJECT_TOKENS: &[&str] = &[
    "400",
    "403",
    "bad_request",
    "forbidden",
    "reject",
    "rejected",
];

/// The site-label suppression markers (fn-wide, lowercase).
const SITE_LABEL_TOKENS: &[&str] = &[
    "sec_fetch",
    "sec-fetch-site",
    "same-site",
    "same_site",
    "cross-site",
    "cross_site",
    "origin_header",
    "referer",
];

/// Detect cross-site-blind strike counting (suggest-only; no edits are
/// produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<StrikeCrossSiteFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<StrikeCrossSiteFinding>) {
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
            // fixture habitat, not production abuse accounting.
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

fn scan_fn(source: &str, block: &syn::Block, findings: &mut Vec<StrikeCrossSiteFinding>) {
    let fn_ts = quote::ToTokens::to_token_stream(block).to_string();
    let fn_lower = fn_ts.to_ascii_lowercase();
    // Gate 1+2: strike vocabulary AND a rejection marker.
    if !STRIKE_TOKENS.iter().any(|t| fn_lower.contains(t)) {
        return;
    }
    if !REJECT_TOKENS.iter().any(|t| fn_lower.contains(t)) {
        return;
    }
    // Gate 3 (fn-wide): a site-label token means the guard classifies.
    if SITE_LABEL_TOKENS.iter().any(|t| fn_lower.contains(t)) {
        return;
    }
    // Report at the strike-vocabulary increment site.
    if let Some((span, text)) = strike_span(block, source) {
        findings.push(StrikeCrossSiteFinding {
            site_text: text,
            site_span: span,
            advice: ADVICE,
        });
    }
}

/// The first strike-vocabulary expression in the fn (the increment site).
fn strike_span(block: &syn::Block, source: &str) -> Option<(Range<usize>, String)> {
    let mut found = None;
    walk_stmts_for_strike(&block.stmts, source, &mut found);
    found
}

fn walk_stmts_for_strike(
    stmts: &[syn::Stmt],
    source: &str,
    found: &mut Option<(Range<usize>, String)>,
) {
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    walk_expr_for_strike_expr(&init.expr, source, found);
                }
            }
            syn::Stmt::Item(item) => walk_item_for_strike(item, source, found),
            syn::Stmt::Expr(e, _) => walk_expr_for_strike_expr(e, source, found),
            syn::Stmt::Macro(_) => {}
        }
    }
}

fn walk_item_for_strike(item: &Item, source: &str, found: &mut Option<(Range<usize>, String)>) {
    match item {
        Item::Fn(f) => walk_stmts_for_strike(&f.block.stmts, source, found),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    walk_stmts_for_strike(&m.block.stmts, source, found);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    walk_item_for_strike(nested, source, found);
                }
            }
        }
        _ => {}
    }
}

fn walk_expr_for_strike_expr(e: &Expr, source: &str, found: &mut Option<(Range<usize>, String)>) {
    if found.is_some() {
        return;
    }
    let ts = quote::ToTokens::to_token_stream(e).to_string();
    if STRIKE_TOKENS
        .iter()
        .any(|t| ts.to_ascii_lowercase().contains(t))
    {
        let span = e.span().byte_range();
        if span.end <= source.len() {
            let text = source[span.clone()].to_string();
            *found = Some((span, text));
        }
        return;
    }
    match e {
        Expr::MethodCall(m) => {
            walk_expr_for_strike_expr(&m.receiver, source, found);
            for a in &m.args {
                walk_expr_for_strike_expr(a, source, found);
            }
        }
        Expr::Call(c) => {
            walk_expr_for_strike_expr(&c.func, source, found);
            for a in &c.args {
                walk_expr_for_strike_expr(a, source, found);
            }
        }
        Expr::If(i) => {
            walk_expr_for_strike_expr(&i.cond, source, found);
            walk_stmts_for_strike(&i.then_branch.stmts, source, found);
            if let Some((_, else_e)) = &i.else_branch {
                walk_expr_for_strike_expr(else_e, source, found);
            }
        }
        Expr::Match(m) => {
            walk_expr_for_strike_expr(&m.expr, source, found);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    walk_expr_for_strike_expr(guard, source, found);
                }
                walk_expr_for_strike_expr(&arm.body, source, found);
            }
        }
        Expr::Block(b) => walk_stmts_for_strike(&b.block.stmts, source, found),
        Expr::Unsafe(u) => walk_stmts_for_strike(&u.block.stmts, source, found),
        Expr::Loop(l) => walk_stmts_for_strike(&l.body.stmts, source, found),
        Expr::While(w) => {
            walk_expr_for_strike_expr(&w.cond, source, found);
            walk_stmts_for_strike(&w.body.stmts, source, found);
        }
        Expr::ForLoop(fl) => {
            walk_expr_for_strike_expr(&fl.expr, source, found);
            walk_stmts_for_strike(&fl.body.stmts, source, found);
        }
        Expr::Try(t) => walk_expr_for_strike_expr(&t.expr, source, found),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::StrikeCrossSiteFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 292 gate — the corpus `buggy_code` MUST fire.
    #[test]
    fn batch292_strike_on_every_rejection_fires() {
        let src = "pub struct Guard { strikes: u32 }\n\n\
             pub enum Verdict { Ok, Rejected }\n\n\
             impl Guard {\n    \
             pub fn inspect(&mut self, body: &str) -> Verdict {\n        \
             if body.contains(\"; cat \") {\n            \
             self.strikes += 1;\n            \
             return Verdict::Rejected;\n        \
             }\n        \
             Verdict::Ok\n    \
             }\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The fixed shape (a site label present) stays silent.
    #[test]
    fn site_labeled_guard_silent() {
        let src = "pub struct Guard { strikes: u32 }\n\n\
             pub fn inspect(&mut self, body: &str, site: Option<&str>) -> bool {\n    \
             if body.contains(\"; cat \") {\n        \
             if site != Some(\"same-site\") {\n            \
             return false; // reject, never count\n        \
             }\n        \
             self.strikes += 1;\n        \
             return false;\n    \
             }\n    \
             true\n}\n";
        assert!(findings(src).is_empty(), "site label present: silent");
    }

    /// A strike counter with no rejection path stays silent.
    #[test]
    fn no_rejection_path_silent() {
        let src = "pub struct Bowler { strikes: u32 }\n\n\
             impl Bowler {\n    \
             pub fn roll(&mut self, pins: u32) {\n        \
             if pins == 0 { self.strikes += 1; }\n    \
             }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A rejection path with no strike vocabulary stays silent.
    #[test]
    fn rejection_without_strikes_silent() {
        let src = "pub fn check(body: &str) -> Result<(), String> {\n    \
             if body.contains(\"../\") { return Err(String::from(\"rejected\")); }\n    \
             Ok(())\n}\n";
        assert!(findings(src).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn t(g: &mut Guard) {\n        \
             g.strikes += 1;\n    \
             }\n}\n";
        assert!(findings(src).is_empty());
    }
}

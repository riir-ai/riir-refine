//! `ipv4-mapped-range-check-unwrap` — SUGGEST-ONLY detector
//! (Batch 292, the hevnsnt/`CVEngine` distill; Research 330 / Plan 296).
//!
//! An `IpAddr` match whose V4 arm runs a range/allowlist/denylist test and
//! whose V6 arm rejects or falls through WITHOUT unwrapping the mapped
//! form: `::ffff:127.0.0.1` is a V6 address whose bits are loopback, so a
//! version-split check waves it through every guard it was meant to fail —
//! the bypass class for SSRF refusals, allowlists and firewall rules.
//! `CVEngine` unwraps `ipv4_mapped` BEFORE any range test in both its
//! resolver and its scanner gate.
//!
//! ## Why suggest-only (never a fix)
//!
//! Some guards deliberately treat all V6 as external (a V6-only admin
//! edge); a mechanical rewrite cannot know the intent. This module ships
//! a parse-only DETECTION walk only: NO `FIXABLE_SEC` membership, NO
//! `collect_edits` arm — the suggestion surfaces through retrieval
//! (`--suggest` / the corpus entry) and review.
//!
//! ## The bounded form
//!
//! ```text
//! IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
//!     Some(v4) => BLOCKED.contains(&v4), // ::ffff:127.0.0.1 IS loopback
//!     None => false,
//! },
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Match-pair gate**: an `Expr::Match` with an `IpAddr::V4(..)` arm
//!   AND an `IpAddr::V6(..)` arm (either binding or `_`); the report
//!   lands at the match.
//! - **Non-unwrapping V6 arm**: the V6 arm's body carries no
//!   `to_ipv4_mapped` call, and the FN carries none either (the
//!   normalize-up-front form suppresses fn-wide).
//! - **Range-test vocabulary gate (the FP control)**: the V4 arm body or
//!   the fn must carry a range-test token (`contains(` /
//!   `is_loopback` / `is_private` / `is_link_local` / `allowlist` /
//!   `blocked` / `nets`) — a debug-print match stays silent.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped;
//!   macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - A folded single-arm match (`IpAddr::V4(v) if matches!(..)` only) is
//!   parse-only unreachable — stated in-rule.
//! - The range-test vocabulary is a gate, not a proof: an exotic guard
//!   spelling its own predicate fires and review decides.
//! - Type inference is not modeled: a non-`IpAddr` enum with
//!   `V4`/`V6`-shaped variant names fires too — the vocabulary gates keep
//!   the class honest, review decides.
//! - Batch 292 / Plan 296 / Research 330 (hevnsnt/`CVEngine` @ `9dd9c441`,
//!   GPL-3.0 — cited as class provenance; the fixture is an original
//!   synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item};

/// One detected mapped-address range-check bypass site.
#[derive(Debug, Clone)]
pub struct Ipv4MappedRangeFinding {
    /// Byte range of the `match` expression.
    pub site_span: Range<usize>,
    /// The match's source text.
    pub site_text: String,
    /// The advice line (the mapped unwrap + the class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "unwrap the mapped form BEFORE the range test: \
`::ffff:127.0.0.1` is a V6 address whose bits are loopback, so a version-split \
check waves it through — `v6.to_ipv4_mapped()` routes mapped addresses down the \
V4 path (and pass the PARSED form onward, never the raw token: \
`IpAddr::from_str` already refuses scope ids; Batch 292, suggest-only)";

/// The range-test vocabulary (token-text substrings, lowercase).
const RANGE_TOKENS: &[&str] = &[
    "contains(",
    "is_loopback",
    "is_private",
    "is_link_local",
    "allowlist",
    "blocked",
    "nets",
];

/// Detect mapped-address range-check bypasses (suggest-only; no edits are
/// produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<Ipv4MappedRangeFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<Ipv4MappedRangeFinding>) {
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
            // fixture habitat, not production address validation.
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

fn scan_fn(source: &str, block: &syn::Block, findings: &mut Vec<Ipv4MappedRangeFinding>) {
    // Fn-wide suppression first: a `to_ipv4_mapped` anywhere means the
    // normalize is present.
    let fn_ts = quote::ToTokens::to_token_stream(block).to_string();
    let fn_lower = fn_ts.to_ascii_lowercase();
    if fn_lower.contains("to_ipv4_mapped") {
        return;
    }
    // Range-test vocabulary gate (the FP control): V4 arm body or fn.
    if !RANGE_TOKENS.iter().any(|t| fn_lower.contains(t)) {
        return;
    }
    // The V4/V6 match-pair walk.
    let mut found: Option<Range<usize>> = None;
    scan_stmts(&block.stmts, source, &mut found);
    if let Some(span) = found {
        findings.push(Ipv4MappedRangeFinding {
            site_text: source[span.clone()].to_string(),
            site_span: span,
            advice: ADVICE,
        });
    }
}

fn scan_stmts(stmts: &[syn::Stmt], source: &str, found: &mut Option<Range<usize>>) {
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    walk_expr_for_pair(&init.expr, source, found);
                }
            }
            syn::Stmt::Item(item) => collect_item_exprs(item, source, found),
            syn::Stmt::Expr(e, _) => walk_expr_for_pair(e, source, found),
            syn::Stmt::Macro(_) => {}
        }
    }
}

fn collect_item_exprs(item: &Item, source: &str, found: &mut Option<Range<usize>>) {
    match item {
        Item::Fn(f) => scan_stmts(&f.block.stmts, source, found),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_stmts(&m.block.stmts, source, found);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_item_exprs(nested, source, found);
                }
            }
        }
        _ => {}
    }
}

fn walk_expr_for_pair(e: &Expr, source: &str, found: &mut Option<Range<usize>>) {
    if found.is_some() {
        return;
    }
    match e {
        Expr::Match(m) => {
            if is_v4_v6_pair(m) {
                let span = e.span().byte_range();
                if span.end <= source.len() {
                    *found = Some(span);
                }
                return;
            }
            walk_expr_for_pair(&m.expr, source, found);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    walk_expr_for_pair(guard, source, found);
                }
                walk_expr_for_pair(&arm.body, source, found);
            }
        }
        Expr::MethodCall(m) => {
            walk_expr_for_pair(&m.receiver, source, found);
            for a in &m.args {
                walk_expr_for_pair(a, source, found);
            }
        }
        Expr::Call(c) => {
            walk_expr_for_pair(&c.func, source, found);
            for a in &c.args {
                walk_expr_for_pair(a, source, found);
            }
        }
        Expr::If(i) => {
            walk_expr_for_pair(&i.cond, source, found);
            scan_stmts(&i.then_branch.stmts, source, found);
            if let Some((_, else_e)) = &i.else_branch {
                walk_expr_for_pair(else_e, source, found);
            }
        }
        Expr::Block(b) => scan_stmts(&b.block.stmts, source, found),
        Expr::Unsafe(u) => scan_stmts(&u.block.stmts, source, found),
        Expr::Loop(l) => scan_stmts(&l.body.stmts, source, found),
        Expr::While(w) => {
            walk_expr_for_pair(&w.cond, source, found);
            scan_stmts(&w.body.stmts, source, found);
        }
        Expr::ForLoop(fl) => {
            walk_expr_for_pair(&fl.expr, source, found);
            scan_stmts(&fl.body.stmts, source, found);
        }
        Expr::Try(t) => walk_expr_for_pair(&t.expr, source, found),
        _ => {}
    }
}

/// The V4/V6 match-pair shape: one `IpAddr::V4(..)` arm + one
/// `IpAddr::V6(..)` arm (binding or `_`), non-unwrapping by the caller's
/// fn-wide gate.
fn is_v4_v6_pair(m: &syn::ExprMatch) -> bool {
    let mut v4 = false;
    let mut v6 = false;
    for arm in &m.arms {
        let name = match &arm.pat {
            // `IpAddr::V4(v4)` — the variant path carries the name; elems
            // are the bindings.
            syn::Pat::TupleStruct(tp) => tp.path.segments.last().map(|s| s.ident.to_string()),
            // `IpAddr::V4` (unit form).
            syn::Pat::Path(pp) => pp.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        };
        match name.as_deref() {
            Some("V4") => v4 = true,
            Some("V6") => v6 = true,
            _ => {}
        }
    }
    v4 && v6
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::Ipv4MappedRangeFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 292 gate — the corpus `buggy_code` MUST fire.
    #[test]
    fn batch292_version_split_bypass_fires() {
        let src = "use std::net::{IpAddr, Ipv4Addr};\n\n\
             const BLOCKED: [Ipv4Addr; 1] = [Ipv4Addr::LOCALHOST];\n\n\
             pub fn is_blocked(ip: IpAddr) -> bool {\n    \
             match ip {\n        \
             IpAddr::V4(v4) => BLOCKED.contains(&v4),\n        \
             IpAddr::V6(_) => false,\n    \
             }\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The fixed shape (`to_ipv4_mapped` in the V6 arm) stays silent.
    #[test]
    fn mapped_unwrap_silent() {
        let src = "use std::net::{IpAddr, Ipv4Addr};\n\n\
             const BLOCKED: [Ipv4Addr; 1] = [Ipv4Addr::LOCALHOST];\n\n\
             pub fn is_blocked(ip: IpAddr) -> bool {\n    \
             match ip {\n        \
             IpAddr::V4(v4) => BLOCKED.contains(&v4),\n        \
             IpAddr::V6(v6) => match v6.to_ipv4_mapped() {\n            \
             Some(v4) => BLOCKED.contains(&v4),\n            \
             None => false,\n        \
             },\n    \
             }\n}\n";
        assert!(findings(src).is_empty(), "unwrap present: silent");
    }

    /// The normalize-up-front form suppresses fn-wide.
    #[test]
    fn normalize_up_front_silent() {
        let src = "use std::net::IpAddr;\n\n\
             pub fn audit(ip: IpAddr) -> bool {\n    \
             let ip = ip.to_ipv4_mapped().map_or(ip, IpAddr::V4);\n    \
             match ip {\n        \
             IpAddr::V4(v4) => v4.is_loopback(),\n        \
             IpAddr::V6(_) => false,\n    \
             }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A debug-print V4/V6 match (no range-test vocabulary) stays silent.
    #[test]
    fn debug_match_silent() {
        let src = "use std::net::IpAddr;\n\n\
             pub fn describe(ip: IpAddr) -> String {\n    \
             match ip {\n        \
             IpAddr::V4(v4) => format!(\"v4 {v4}\"),\n        \
             IpAddr::V6(v6) => format!(\"v6 {v6}\"),\n    \
             }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A V4-only match (no V6 arm) is not the pair shape.
    #[test]
    fn v4_only_match_silent() {
        let src = "use std::net::{IpAddr, Ipv4Addr};\n\n\
             pub fn audit(ip: IpAddr) -> bool {\n    \
             matches!(ip, IpAddr::V4(v4) if v4.is_loopback())\n}\n";
        assert!(findings(src).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn t(ip: std::net::IpAddr) -> bool {\n        \
             match ip {\n            \
             std::net::IpAddr::V4(v) => v.is_loopback(),\n            \
             std::net::IpAddr::V6(_) => false,\n        \
             }\n    \
             }\n}\n";
        assert!(findings(src).is_empty());
    }
}

//! `webhook-signature-over-raw-bytes` — SUGGEST-ONLY detector
//! (Batch 258, the cloudflare/security-audit-skill distill; Research 292 /
//! Plan 262).
//!
//! A signature/HMAC verification computed over a RE-SERIALIZED body
//! (`serde_json::to_string(&parsed)` / `to_vec`) instead of the exact
//! received bytes: canonicalization drift (key order, whitespace, number
//! format) lets an attacker reshape a payload and keep the signature
//! check passing — the "untrusted producer treated as control plane" /
//! "envelope and payload identity mismatch" class. The verify must cover
//! the RAW bytes the sender signed, before or alongside any parse.
//!
//! ## Why suggest-only (never a fix)
//!
//! Which key, which fields, and the timestamp/nonce binding are
//! per-scheme decisions; rewriting the coverage is judgment-bearing.
//! This module ships a parse-only DETECTION walk only: NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm — the suggestion surfaces through
//! retrieval (`--suggest` / the corpus entry) and review.
//!
//! ## The bounded form
//!
//! ```text
//! verify_sig(secret, raw)          // over the exact received bytes
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Verify-vocabulary callee, parse-provable**: the callee's last path
//!   segment (or method name) splits into `_`-tokens, and any token is
//!   `verify` / `verified` / `hmac`, or the tokens contain a signature
//!   word (`sig` / `signature` / `sign`) AND a checking word (`check` /
//!   `validate` / `valid`). A plain `send(out)` never fires.
//! - **The re-serialization is the trigger**: an argument that IS
//!   (or is bound in-fn from) `serde_json::to_string` / `to_vec` /
//!   `to_string_pretty` / `to_vec_pretty`. A verify over a raw binding,
//!   a literal, or an HMAC-output binding is silent.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped;
//!   macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - **Canonical-signature schemes fire by design**: a protocol that
//!   deliberately signs the CANONICAL re-serialization (JWS-style) is
//!   flagged too, and review decides — the rule flags the COVERAGE
//!   question, not a definite bug.
//! - The re-serialization must happen in the SAME fn as the verify;
//!   cross-fn coverage questions are not connected parse-only.
//! - `to_writer` forms and non-serde re-encodings are unreached.
//! - **Distinct from `constant-time-secret-compare` (owned)**: that rule
//!   owns the COMPARE primitive (timing); this owns WHICH BYTES the
//!   signature covers (coverage) — different axes, both can fire on one
//!   fn with disjoint spans.
//! - Batch 258 / Plan 262 / Research 292 (cloudflare/security-audit-skill
//!   @ `c1c8a8c`, MIT — cited as class provenance; the fixture is an
//!   original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item, Stmt};

/// One detected verify-over-re-serialized-bytes flow.
#[derive(Debug, Clone)]
pub struct WebhookRawFinding {
    /// Byte range of the verify call.
    pub call_span: Range<usize>,
    /// The verify call's source text.
    pub call_text: String,
    /// The re-serialized argument (the `serde_json::to_*` call text or the
    /// bound local's name).
    pub serialized_arg: String,
    /// The advice line (verify over the raw bytes + the failure class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "verify the signature over the EXACT received bytes \
(the raw request body), before or alongside any parse — a signature computed \
over a re-serialized value (`serde_json::to_string`/`to_vec` of a just-parsed \
body) verifies attacker-reshaped payloads: key order, whitespace, and \
number-format drift make a forged body pass (envelope/payload identity \
mismatch; Batch 258, suggest-only)";

/// The `serde_json::to_*` constructors (last-two-segments pairs).
const RE_SERIALIZERS: &[(&str, &str)] = &[
    ("serde_json", "to_string"),
    ("serde_json", "to_vec"),
    ("serde_json", "to_string_pretty"),
    ("serde_json", "to_vec_pretty"),
];

/// Detect signature verifications computed over re-serialized bytes
/// (suggest-only; no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<WebhookRawFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<WebhookRawFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, f, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_shaped(source, &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production verification paths.
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

fn scan_fn(source: &str, f: &syn::ItemFn, findings: &mut Vec<WebhookRawFinding>) {
    scan_shaped(source, &f.block, findings);
}

fn scan_shaped(source: &str, block: &syn::Block, findings: &mut Vec<WebhookRawFinding>) {
    // Pass 1: locals bound (transparently) from a re-serialization.
    let mut reser: std::collections::HashMap<String, String> = std::collections::HashMap::new(); // local ident -> the to_* call text
    collect_reserialized_locals(block, &mut reser);

    // Pass 2: verify-vocabulary calls carrying a re-serialized argument.
    let mut reported: std::collections::HashSet<Range<usize>> = std::collections::HashSet::new();
    scan_verify_calls(source, block, &reser, &mut reported, findings);
}

fn collect_reserialized_locals(
    block: &syn::Block,
    out: &mut std::collections::HashMap<String, String>,
) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    if let Some(text) = find_re_serializer(&init.expr) {
                        let mut pats = Vec::new();
                        collect_pat_idents(&l.pat, &mut pats);
                        for p in pats {
                            out.entry(p).or_insert(text.clone());
                        }
                    }
                }
            }
            Stmt::Item(item) => collect_reserialized_locals_item(item, out),
            Stmt::Expr(e, _) => collect_reserialized_locals_expr(e, out),
            Stmt::Macro(_) => {}
        }
    }
}

fn collect_reserialized_locals_item(
    item: &Item,
    out: &mut std::collections::HashMap<String, String>,
) {
    match item {
        Item::Fn(f) => collect_reserialized_locals(&f.block, out),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    collect_reserialized_locals(&m.block, out);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_reserialized_locals_item(nested, out);
                }
            }
        }
        _ => {}
    }
}

fn collect_reserialized_locals_expr(e: &Expr, out: &mut std::collections::HashMap<String, String>) {
    match e {
        Expr::Block(b) => collect_reserialized_locals(&b.block, out),
        Expr::Unsafe(u) => collect_reserialized_locals(&u.block, out),
        Expr::Loop(l) => collect_reserialized_locals(&l.body, out),
        Expr::While(w) => {
            collect_reserialized_locals_expr(&w.cond, out);
            collect_reserialized_locals(&w.body, out);
        }
        Expr::If(i) => {
            collect_reserialized_locals_expr(&i.cond, out);
            collect_reserialized_locals(&i.then_branch, out);
            if let Some((_, else_e)) = &i.else_branch {
                collect_reserialized_locals_expr(else_e, out);
            }
        }
        Expr::Match(m) => {
            for arm in &m.arms {
                collect_reserialized_locals_expr(&arm.body, out);
            }
        }
        _ => {}
    }
}

/// If `e` IS a `serde_json::to_*` call (through Try/refs/parens and the
/// unwrap family), return the call's text.
fn find_re_serializer(e: &Expr) -> Option<String> {
    let inner = match e {
        Expr::Try(t) => &t.expr,
        Expr::Paren(p) => &p.expr,
        Expr::Reference(r) => &r.expr,
        Expr::MethodCall(m) if is_unwrap_family(&m.method.to_string()) => m.receiver.as_ref(),
        _ => e,
    };
    match inner {
        Expr::Try(t) => find_re_serializer(&t.expr),
        Expr::Paren(p) => find_re_serializer(&p.expr),
        Expr::Reference(r) => find_re_serializer(&r.expr),
        Expr::MethodCall(m) if is_unwrap_family(&m.method.to_string()) => {
            find_re_serializer(m.receiver.as_ref())
        }
        Expr::Call(c) => {
            if let Expr::Path(p) = c.func.as_ref() {
                if p.qself.is_none() && p.path.segments.len() >= 2 {
                    let pair = (
                        p.path.segments[p.path.segments.len() - 2].ident.to_string(),
                        p.path.segments[p.path.segments.len() - 1].ident.to_string(),
                    );
                    if RE_SERIALIZERS.contains(&(pair.0.as_str(), pair.1.as_str())) {
                        return Some(quote::ToTokens::to_token_stream(c).to_string());
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn is_unwrap_family(m: &str) -> bool {
    matches!(
        m,
        "unwrap" | "expect" | "unwrap_or_default" | "unwrap_or_else"
    )
}

/// Is this callee path/method name in the verify vocabulary?
fn is_verify_callee(name: &str) -> bool {
    let mut sig_word = false;
    let mut check_word = false;
    for token in name.split(['_', '-', ' ']) {
        match token {
            "verify" | "verified" | "hmac" => return true,
            "sig" | "signature" | "sign" => sig_word = true,
            "check" | "validate" | "valid" => check_word = true,
            _ => {}
        }
    }
    sig_word && check_word
}

fn scan_verify_calls(
    source: &str,
    block: &syn::Block,
    reser: &std::collections::HashMap<String, String>,
    reported: &mut std::collections::HashSet<Range<usize>>,
    findings: &mut Vec<WebhookRawFinding>,
) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_verify_expr(source, &init.expr, reser, reported, findings);
                }
            }
            Stmt::Item(item) => scan_verify_item(source, item, reser, reported, findings),
            Stmt::Expr(e, _) => scan_verify_expr(source, e, reser, reported, findings),
            Stmt::Macro(_) => {}
        }
    }
}

fn scan_verify_item(
    source: &str,
    item: &Item,
    reser: &std::collections::HashMap<String, String>,
    reported: &mut std::collections::HashSet<Range<usize>>,
    findings: &mut Vec<WebhookRawFinding>,
) {
    match item {
        Item::Fn(f) => scan_verify_calls(source, &f.block, reser, reported, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_verify_calls(source, &m.block, reser, reported, findings);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    scan_verify_item(source, nested, reser, reported, findings);
                }
            }
        }
        _ => {}
    }
}

fn scan_verify_expr(
    source: &str,
    e: &Expr,
    reser: &std::collections::HashMap<String, String>,
    reported: &mut std::collections::HashSet<Range<usize>>,
    findings: &mut Vec<WebhookRawFinding>,
) {
    let callee_name: Option<String> = match e {
        Expr::Call(c) => match c.func.as_ref() {
            Expr::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        },
        Expr::MethodCall(m) => Some(m.method.to_string()),
        _ => None,
    };
    if let Some(name) = callee_name {
        if is_verify_callee(&name) {
            let args: Vec<&Expr> = match e {
                Expr::Call(c) => c.args.iter().collect(),
                Expr::MethodCall(m) => {
                    let mut v: Vec<&Expr> = vec![m.receiver.as_ref()];
                    v.extend(m.args.iter());
                    v
                }
                _ => Vec::new(),
            };
            for a in args {
                // Inline re-serialization argument.
                if let Some(text) = find_re_serializer(a) {
                    push(source, e, &text, reported, findings);
                    break;
                }
                // Bound-local re-serialization argument.
                if let Expr::Path(p) = strip_refs(a) {
                    if p.qself.is_none() && p.path.segments.len() == 1 {
                        let id = p.path.segments[0].ident.to_string();
                        if let Some(text) = reser.get(&id) {
                            push(source, e, text, reported, findings);
                            break;
                        }
                    }
                }
            }
        }
    }
    walk_children(e, &mut |child| {
        scan_verify_expr(source, child, reser, reported, findings);
    });
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

fn push(
    source: &str,
    call: &Expr,
    serialized_arg: &str,
    reported: &mut std::collections::HashSet<Range<usize>>,
    findings: &mut Vec<WebhookRawFinding>,
) {
    let span = call.span().byte_range();
    if span.end > source.len() || !reported.insert(span.clone()) {
        return;
    }
    findings.push(WebhookRawFinding {
        call_text: source[span.clone()].to_string(),
        call_span: span,
        serialized_arg: serialized_arg.to_string(),
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

    fn findings(src: &str) -> Vec<super::WebhookRawFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 258 gate — the corpus `buggy_code` MUST fire.
    #[test]
    fn batch258_reserialized_verify_fires() {
        let src = "pub fn verify_webhook(secret: &[u8], raw: &[u8]) -> bool {\n    \
             let event: serde_json::Value = serde_json::from_slice(raw).unwrap_or_default();\n    \
             let canonical = serde_json::to_vec(&event).unwrap_or_default();\n    \
             verify_sig(secret, &canonical)\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert!(
            f[0].call_text.starts_with("verify_sig"),
            "{:?}",
            f[0].call_text
        );
        assert!(
            f[0].serialized_arg.contains("to_vec"),
            "{:?}",
            f[0].serialized_arg
        );
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// Verify over the raw binding (the fixed form) stays silent.
    #[test]
    fn raw_verify_silent() {
        let src = "pub fn verify_webhook(secret: &[u8], raw: &[u8]) -> bool {\n    \
             verify_sig(secret, raw)\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A re-serialization OUTSIDE a verify-vocabulary callee is out of
    /// scope (the send path, not the verify path).
    #[test]
    fn re_serialize_without_verify_silent() {
        let src = "pub fn relay(event: &serde_json::Value) {\n    \
             let out = serde_json::to_vec(event).unwrap_or_default();\n    \
             send(&out);\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A verify over an HMAC-output binding (not a re-serialization) is
    /// silent.
    #[test]
    fn hmac_output_binding_silent() {
        let src = "pub fn check(key: &[u8], body: &[u8]) -> bool {\n    \
             let mac = hmac_digest(key, body);\n    \
             hmac_verify(key, &mac)\n}\n";
        assert!(findings(src).is_empty());
    }

    /// The inline re-serialization argument fires.
    #[test]
    fn inline_re_serialize_arg_fires() {
        let src = "pub fn v(event: &serde_json::Value, key: &[u8]) -> bool {\n    \
             verify_hmac(key, &serde_json::to_vec(event).unwrap())\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "inline form fires: {f:?}");
    }

    /// Method-call verify form fires on the re-serialized receiver-arg.
    #[test]
    fn method_form_fires() {
        let src = "pub fn v(verifier: &Sig, raw: &[u8]) -> bool {\n    \
             let event: serde_json::Value = serde_json::from_slice(raw).unwrap_or_default();\n    \
             let canonical = serde_json::to_vec(&event).unwrap_or_default();\n    \
             verifier.check_signature(&canonical)\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "method form fires: {f:?}");
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn t(raw: &[u8]) {\n        \
             let v: serde_json::Value = serde_json::from_slice(raw).unwrap_or_default();\n        \
             let c = serde_json::to_vec(&v).unwrap_or_default();\n        \
             let _ = verify_sig(&c);\n    \
             }\n}\n";
        assert!(findings(src).is_empty());
    }
}

//! `regex-backtracking-engine-untrusted-pattern` — SUGGEST-ONLY detector
//! (Batch 351, the 0sec-labs/foxguard distill; Research 394 / Plan 361).
//! The `ReDoS` family seed — zero prior corpus owners.
//!
//! A regex pattern with a NESTED QUANTIFIER — a quantified group that
//! itself contains a quantifier, `(a+)+` / `(a*)*` / `(\d+)+` — executed
//! by a BACKTRACKING engine over attacker-shaped input degrades
//! catastrophically: the engine splits candidates exponentially over a
//! near-miss input (`aaaaaaaaaaaaaaaaaaaaaaaab`) and pins the worker
//! (CWE-1333, `ReDoS`).
//!
//! ## The source anchor (0sec-labs/foxguard @ `fbeab16`)
//!
//! `src/rules/javascript.rs` `NoUnsafeRegex` + `js_redos_re`, verbatim
//! at the pin: "Only flag a quantified group that itself contains a
//! quantifier, i.e. a genuine nested quantifier like `(a+)+` / `(a*)*` /
//! `(\d+)+`. Plain alternation groups such as `(a|b)+` are not
//! catastrophic on their own, so we no longer flag them (false-positive
//! reduction)." The detector shape transcribes: a nested-quantifier
//! shape gate over the pattern literal — never a plain-alternation
//! flag.
//!
//! ## THE ENGINE SPLIT IS THE DECISION CONTENT (the Rust-specific law)
//!
//! The `regex` crate (RE2-derived) is LINEAR-TIME by construction —
//! the anti-regime in-rule: **never flag a `regex::Regex` on pattern
//! shape**; the shape gate fires only where a backtracking engine is
//! selected: `fancy_regex::Regex` (backtracking fallback for
//! lookarounds/backreferences) and `pcre2::Regex`. The corpus carries
//! fancy-regex only as a fix spelling (`ascii-token-unicode-word-
//! boundary-scan-miss`) — never before as a hazard surface.
//!
//! ## Why suggest-only (never a fix)
//!
//! The repair is a triple choice — prefer the linear engine for
//! untrusted patterns, bound the input length, or strip the
//! backtracking features (no lookarounds/backreferences) when the
//! pattern is user-supplied — each a design + dependency decision. NO
//! `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - The engine gate is fn-scope vocabulary (`fancy_regex` / `pcre2`
//!   tokens): a backtracking engine imported under an alias one module
//!   away is an honest NO.
//! - A nested-quantifier pattern on a TRUSTED, length-bounded input
//!   fires and review decides (the shape is the hazard's precondition,
//!   not its proof — the fix may legitimately be the input bound).
//! - Catastrophic shapes BEYOND nested quantifiers (evil-number
//!   regressions, unicode-collation explosions) are out of class — the
//!   source's own detector scopes to the nested-quantifier shape.
//! - The gate is explicit-path precedence: a crate-prefixed
//!   `regex::Regex::new` site is NEVER flagged (the anti-regime, precise
//!   form), while a bare `Regex::new` in a file importing a backtracking
//!   engine still fires — a parse-only scan cannot bind the bare name to
//!   its crate across module scopes, so it fires and review decides.
//!   Name the crate explicitly at the compile site to claim the
//!   exemption.
//! - Detector: `proposers::redos_engine_pattern`. Fixture:
//!   `fixtures/b351_fixtures.rs` F3. Batch 351 / Plan 361 / Research 394
//!   (0sec-labs/foxguard @ `fbeab16`, MIT OR Apache-2.0 — cited as class
//!   provenance; the fixture is an original synthesis, standalone-compiled
//!   AND executed before entry).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item};

/// One detected backtracking-engine nested-quantifier site.
#[derive(Debug, Clone)]
pub struct RedosFinding {
    /// Byte range of the pattern-literal expression.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the engine split).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a nested-quantifier pattern (`(a+)+`, `(a*)*`, \
`(\\d+)+`) under a BACKTRACKING engine (fancy-regex / pcre2) degrades \
catastrophically on near-miss input — the engine splits candidates \
exponentially (CWE-1333). The `regex` crate is linear-time by \
construction — prefer it for untrusted patterns, bound the input length, \
or strip the backtracking features (no lookarounds/backreferences) when \
the pattern is user-supplied (Batch 351, suggest-only)";

/// The backtracking-engine vocabulary (the engine gate).
const BACKTRACKING_ENGINES: &[&str] = &["fancy_regex", "fancy-regex", "pcre2"];

/// Detect nested-quantifier patterns under backtracking engines
/// (suggest-only; no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<RedosFinding> {
    let mut findings = Vec::new();
    // File-level import context: the engine lives in the `use` line.
    let import_text = file
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Use(_) => Some(quote::ToTokens::to_token_stream(i).to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    for item in &file.items {
        collect_in_item(source, item, &import_text, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, import_text: &str, findings: &mut Vec<RedosFinding>) {
    match item {
        Item::Fn(f) => {
            let fn_ts = quote::ToTokens::to_token_stream(f).to_string();
            let scope = format!("{fn_ts} {import_text}");
            scan_fn_text(source, f, &scope, findings);
        }
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    let fn_ts = quote::ToTokens::to_token_stream(m).to_string();
                    let scope = format!("{fn_ts} {import_text}");
                    scan_fn_text(source, &m_to_item_fn(m), &scope, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production pattern compilation.
            if m.attrs.iter().any(|a| {
                a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test")
            }) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, import_text, findings);
                }
            }
        }
        _ => {}
    }
}

fn scan_fn_text(source: &str, f: &syn::ItemFn, scope: &str, findings: &mut Vec<RedosFinding>) {
    // THE ENGINE GATE (the anti-regime): a backtracking-engine token in
    // the fn's scope (fn text + the file's `use` trees, whitespace-
    // compacted — token streams render with spaces) arms the shape gate;
    // the Call arm then applies explicit-path precedence, so an
    // explicitly crate-prefixed `regex::Regex::new` site is NEVER
    // flagged, whatever the file imports. A bare `Regex::new` in a
    // fancy-importing file still fires (review decides — the bare name
    // cannot be scope-bound parse-only).
    let compact: String = scope.chars().filter(|c| !c.is_whitespace()).collect();
    let engine_in_scope = BACKTRACKING_ENGINES.iter().any(|t| {
        let bare = t.replace('-', "_");
        compact.contains(&bare)
                // The crate-path spelling ("fancy-regex" in Cargo.toml
                // vocabulary, "fancy_regex::" in code) is what the doc
                // names; the hyphen form rides the compacted text too.
                || compact.contains(t)
    });
    if !engine_in_scope {
        return;
    }
    scan_stmts_scoped(&f.block.stmts, source, engine_in_scope, findings);
}

/// A method's block as a synthetic `ItemFn` (the `ImplItem::Fn` shape has
/// no name/signature of its own here — the scan only needs the body).
fn m_to_item_fn(m: &syn::ImplItemFn) -> syn::ItemFn {
    syn::parse_quote! {
        fn synthetic() #m.block
    }
}

fn scan_stmts_scoped(
    stmts: &[syn::Stmt],
    source: &str,
    engine_in_scope: bool,
    findings: &mut Vec<RedosFinding>,
) {
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_expr_scoped(&init.expr, source, engine_in_scope, findings);
                }
            }
            syn::Stmt::Expr(e, _) => {
                scan_expr_scoped(e, source, engine_in_scope, findings);
            }
            _ => {}
        }
    }
}

fn scan_expr_scoped(
    e: &Expr,
    source: &str,
    engine_in_scope: bool,
    findings: &mut Vec<RedosFinding>,
) {
    match e {
        // The compile site: `fancy_regex::Regex::new("pattern")` /
        // `pcre2::Regex::new("pattern")` — a CALL whose func path names
        // the engine (or whose scope imports it under an alias) and
        // whose first argument is the pattern literal.
        Expr::Call(c) => {
            let func_ts = quote::ToTokens::to_token_stream(&c.func).to_string();
            let func_compact: String = func_ts.chars().filter(|ch| !ch.is_whitespace()).collect();
            let func_lower = func_compact.to_lowercase();
            // Explicit-path precedence (the anti-regime's precise
            // form): the callee's FIRST path segment names the engine.
            // A crate-prefixed path whose first segment is `regex`
            // (`regex::Regex::new` — 3+ segments) is the linear engine
            // named explicitly — exempt whatever the file imports. The
            // bare `Regex::new` spelling is also first-segment "regex"
            // but only 2 segments: it binds per file scope, so it keeps
            // the import-scope gate (fires when a backtracking engine is
            // imported anywhere; review decides — the honest limit).
            let segs: Vec<&str> = func_compact.split("::").collect();
            let first_seg = segs.first().copied().unwrap_or("").to_lowercase();
            let explicit_backtracking = BACKTRACKING_ENGINES
                .iter()
                .any(|t| first_seg == t.replace('-', "_"));
            let explicit_linear = first_seg == "regex" && segs.len() > 2;
            let engine_named = explicit_backtracking || (engine_in_scope && !explicit_linear);
            let is_new = func_lower.ends_with("::new");
            if engine_named && is_new {
                if let Some(Expr::Lit(lit)) = c.args.first() {
                    if let syn::Lit::Str(s) = &lit.lit {
                        let pattern = s.value();
                        if nested_quantifier(&pattern) {
                            let byte_range = e.span().byte_range();
                            findings.push(RedosFinding {
                                site_span: byte_range.clone(),
                                site_text: source.get(byte_range).unwrap_or("").to_string(),
                                advice: ADVICE,
                            });
                            return;
                        }
                    }
                }
            }
            // Recurse (a builder chain around the compile site).
            scan_expr_scoped(&c.func, source, engine_in_scope, findings);
            for a in &c.args {
                scan_expr_scoped(a, source, engine_in_scope, findings);
            }
        }
        Expr::MethodCall(mc) => {
            scan_expr_scoped(&mc.receiver, source, engine_in_scope, findings);
            for a in &mc.args {
                scan_expr_scoped(a, source, engine_in_scope, findings);
            }
        }
        Expr::Macro(m) => {
            let name = m
                .mac
                .path
                .segments
                .last()
                .map_or(String::new(), |s| s.ident.to_string());
            // Macro-token hygiene: only the pattern-taking macros.
            if name == "fancy_regex" || name == "pcre2" {
                let pattern = m.mac.tokens.to_string();
                if nested_quantifier(&pattern) {
                    let byte_range = e.span().byte_range();
                    findings.push(RedosFinding {
                        site_span: byte_range.clone(),
                        site_text: source.get(byte_range).unwrap_or("").to_string(),
                        advice: ADVICE,
                    });
                }
            }
        }
        _ => {}
    }
}

/// The source's `js_redos_re` shape law, as a scan over the pattern
/// text: a quantified group that itself contains a quantifier — a
/// parenthesised body carrying a quantifier, with the group itself
/// quantified. Plain alternation groups are NOT catastrophic on their
/// own (the source's FP reduction) and do not fire.
fn nested_quantifier(pattern: &str) -> bool {
    let b = pattern.as_bytes();
    let mut depth = 0usize;
    let mut group_start: Option<usize> = None;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => {
                i += 1; // skip the escaped char
            }
            b'(' => {
                depth += 1;
                if depth == 1 {
                    group_start = Some(i + 1);
                }
            }
            b')' if depth > 0 => {
                let start = group_start.unwrap_or(i);
                let body = &pattern[start..i];
                let body_quantified =
                    body.contains('+') || body.contains('*') || body.contains('{');
                // The group is quantified iff the NEXT char quantifies it.
                let next = b.get(i + 1).copied();
                let group_quantified = matches!(next, Some(b'+' | b'*' | b'{'));
                if body_quantified && group_quantified {
                    return true;
                }
                depth = depth.saturating_sub(1);
                group_start = None;
            }
            _ => {}
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(src_text: &str) -> Vec<RedosFinding> {
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
            .find(|r| r.rule_id == "regex-backtracking-engine-untrusted-pattern")
            .expect("rule present");
        assert!(
            !findings(&rule.buggy_code).is_empty(),
            "the proposer must fire on its own buggy fixture"
        );
    }

    /// Batch 351 gate — the corpus `fixed_code` MUST stay silent (the
    /// fix moves to the linear engine — the engine gate suppresses).
    #[test]
    fn corpus_fixed_fixture_is_silent() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "regex-backtracking-engine-untrusted-pattern")
            .expect("rule present");
        assert!(
            findings(&rule.fixed_code).is_empty(),
            "the fixed fixture must stay silent"
        );
    }

    /// THE ANTI-REGIME: the same nested-quantifier pattern under the
    /// LINEAR `regex` crate is NEVER flagged.
    #[test]
    fn linear_engine_never_flagged() {
        let src_text = "pub fn scan(input: &str) -> bool {\n    \
             let re = regex::Regex::new(\"(a+)+$\").unwrap();\n    \
             re.is_match(input)\n}\n";
        assert!(
            findings(src_text).is_empty(),
            "the linear engine is clean on ANY pattern shape (the anti-regime)"
        );
    }

    /// The nested-quantifier shape fires under fancy-regex; the plain
    /// alternation group does NOT (the source's FP reduction).
    #[test]
    fn nested_fires_alternation_not() {
        let nested = "pub fn scan(input: &str) -> bool {\n    \
             let re = fancy_regex::Regex::new(\"^(a+)+$\").unwrap();\n    \
             re.is_match(input).unwrap_or(false)\n}\n";
        assert_eq!(findings(nested).len(), 1, "the nested quantifier fires");
        let alternation = "pub fn scan(input: &str) -> bool {\n    \
             let re = fancy_regex::Regex::new(\"^(cat|dog)+$\").unwrap();\n    \
             re.is_match(input).unwrap_or(false)\n}\n";
        assert!(
            findings(alternation).is_empty(),
            "plain alternation groups are not catastrophic (the source's FP reduction)"
        );
    }

    /// pcre2 is the second backtracking surface.
    #[test]
    fn pcre2_fires() {
        let src_text = "pub fn scan(input: &str) -> bool {\n    \
             let re = pcre2::Regex::new(\"(\\\\d+)+x\").unwrap();\n    \
             re.is_match(input)\n}\n";
        assert_eq!(findings(src_text).len(), 1);
    }

    /// A linear-engine fn with no pattern at all stays silent (no false
    /// fire on ordinary regex code).
    #[test]
    fn ordinary_regex_code_silent() {
        let src_text = "pub fn split_words(s: &str) -> Vec<&str> {\n    \
             s.split_whitespace().collect()\n}\n";
        assert!(findings(src_text).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src_text = "#[cfg(test)]\nmod tests {\n    \
             fn t() -> bool {\n        \
             let re = fancy_regex::Regex::new(\"(a+)+$\").unwrap();\n        \
             re.is_match(\"aaa\").unwrap_or(false)\n    }\n}\n";
        assert!(findings(src_text).is_empty());
    }
}

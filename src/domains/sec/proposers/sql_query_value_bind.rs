//! `sql-query-value-interpolation-unbound` — SUGGEST-ONLY detector
//! (Batch 351, the 0sec-labs/foxguard distill; Research 394 / Plan 361).
//!
//! A DYNAMIC VALUE interpolated into a SQL string (`format!("SELECT …
//! WHERE x = '{v}'")`, a `+` concat of a non-literal operand) flowing
//! into a SQL-method sink (`query` / `execute` / `raw_sql` / `sql_query`
//! vocabulary — the source's own sink set): the value rides statement
//! TEXT instead of a bind parameter, so a crafted value (a quote, a
//! semicolon, a comment marker) becomes executable SQL — the
//! structural-SQLI CWE-89 VALUE face.
//!
//! ## The source anchor (0sec-labs/foxguard @ `fbeab16`)
//!
//! `src/rules/rust_lang.rs` `NoSqlInjection` + `format_has_interpolation`,
//! verbatim at the pin: "Only flag a `format!` that actually interpolates
//! a value. A static `format!(\"SELECT 1\")` with no `{}` placeholder
//! cannot inject anything and was a false positive." The sink set is
//! their `rust_sql_methods_re`: `(query|sql_query|execute|raw_sql)` —
//! carried here as fn-scope vocabulary, never a line regex. Fix advice
//! verbatim: "SQL query built with format! macro — use parameterized
//! queries".
//!
//! ## THE IN-RULE ADJUDICATION (mandatory, both directions) vs
//! `sql-identifier-interpolation-unallowlisted` (Batch 350)
//!
//! Same sink family, OPPOSITE position — the jwt-rules precedent:
//! - THAT rule owns the IDENTIFIER position (`ORDER BY {key}`,
//!   `SELECT {col} FROM`, dynamic `WHERE {key} =` filter builders):
//!   values bind, identifiers cannot ("you can't bind an identifier"),
//!   so the repair is the ALLOWLIST and the bind chain NEVER suppresses.
//! - THIS rule owns the VALUE position (`WHERE x = '{v}'`,
//!   `INSERT … VALUES ('{v}')`, `LIKE '%{q}%'`): values CAN bind —
//!   the repair is the BIND PARAMETER, and an allowlist marker in the
//!   fn does NOT suppress by itself (an allowlist on one column leaves
//!   the value slots of the same statement open). What DOES suppress:
//!   a `.bind(` call in the fn (the fix's own vocabulary) or an
//!   interpolation whose operand is provably constant.
//! - Cross-fire discipline: an identifier-position literal (ORDER BY /
//!   GROUP BY adjacency) is THAT rule's finding — this detector refuses
//!   it (the placeholder must sit in a quoted VALUE slot or a bare-value
//!   position, never immediately after an ORDER BY / GROUP BY keyword).
//!
//! ## The anti-regimes carried (both quoted at the pin)
//!
//! - A `format!` with NO placeholder is silent ("a static
//!   `format!(\"SELECT 1\")` … cannot inject anything").
//! - A trusted-constant operand (`ORDER BY {key}` post-allowlist — the
//!   B350 fixed fixture's own shape) is silent here too: the constant
//!   arm's rule is the allowlist rule's, not this one's. Only
//!   non-literal operands count as flowing.
//!
//! ## Why suggest-only (never a fix)
//!
//! The bind rewrite renumbers placeholders and changes the call's
//! argument list (`.bind(v)` chains) — the mechanical rewrite would
//! have to invent parameter ordering the crate's API decides. NO
//! `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - SQL assembled across helpers is unconnected parse-only — fn-level
//!   gates only (the B350 law transfers whole).
//! - Typed query builders (sea-query/diesel) never carry a SQL literal —
//!   an honest NO by construction.
//! - The quoted-slot adjacency is a heuristic: a placeholder in an
//!   unquoted value position (`WHERE x = {v}`) fires too and review
//!   decides.
//! - Detector: `proposers::sql_query_value_bind`. Fixture:
//!   `fixtures/b351_fixtures.rs` F1. Batch 351 / Plan 361 / Research 394
//!   (0sec-labs/foxguard @ `fbeab16`, MIT OR Apache-2.0 — cited as class
//!   provenance; the fixture is an original synthesis, standalone-compiled
//!   AND executed before entry).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item};

/// One detected unbound SQL value-interpolation site.
#[derive(Debug, Clone)]
pub struct SqlValueFinding {
    /// Byte range of the interpolating expression.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the bind-parameter law).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a dynamic value interpolated into a SQL string \
rides statement TEXT, so a crafted value (`x' OR 1=1 --`) becomes \
executable SQL (CWE-89 value face). Pass the value as a bind parameter \
(`sqlx::query(&sql).bind(v)` / a `?N` or `$N` placeholder) — values bind, \
so the statement text stays fixed (Batch 351, suggest-only)";

/// The SQL sink vocabulary (the source's `rust_sql_methods_re`, as
/// fn-scope tokens).
const SQL_SINKS: &[&str] = &["query", "sql_query", "execute", "raw_sql"];

/// Detect unbound SQL value interpolation (suggest-only; no edits are
/// produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<SqlValueFinding> {
    let mut findings = Vec::new();
    // The constant-binding law (the source's `collect_constant_bindings`
    // transcription): identifiers proven to hold only constant literals
    // are not dynamic. File-level `const` names feed the suppression in
    // `value_slot_finding`.
    let mut const_names = std::collections::HashSet::new();
    collect_const_names(&file.items, &mut const_names);
    for item in &file.items {
        collect_in_item(source, item, &const_names, &mut findings);
    }
    findings
}

/// File-level `const` item names, including consts inside non-test
/// modules (the same `#[cfg(test)]` guard the scan applies).
fn collect_const_names(items: &[Item], names: &mut std::collections::HashSet<String>) {
    for item in items {
        match item {
            Item::Const(c) => {
                names.insert(c.ident.to_string());
            }
            Item::Mod(m) => {
                if m.attrs.iter().any(|a| {
                    a.path().is_ident("cfg")
                        && a.meta.to_token_stream().to_string().contains("test")
                }) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    collect_const_names(nested, names);
                }
            }
            _ => {}
        }
    }
}

fn collect_in_item(
    source: &str,
    item: &Item,
    const_names: &std::collections::HashSet<String>,
    findings: &mut Vec<SqlValueFinding>,
) {
    match item {
        Item::Fn(f) => scan_fn(source, &f.block, const_names, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(source, &m.block, const_names, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production query paths.
            if m.attrs.iter().any(|a| {
                a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test")
            }) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, const_names, findings);
                }
            }
        }
        _ => {}
    }
}

fn scan_fn(
    source: &str,
    block: &syn::Block,
    const_names: &std::collections::HashSet<String>,
    findings: &mut Vec<SqlValueFinding>,
) {
    let fn_ts = quote::ToTokens::to_token_stream(block).to_string();
    // Token streams render with spaces — compact before the vocabulary
    // checks.
    let compact: String = fn_ts.chars().filter(|c| !c.is_whitespace()).collect();
    // Sink gate: a SQL-method token in the fn (the source's sink set).
    if !SQL_SINKS.iter().any(|s| compact.contains(s)) {
        return;
    }
    // Suppression, fn-wide (the fix's own vocabulary): a `.bind(` call
    // means values already ride parameters — the mixed-statement case is
    // the B350 rule's (its identifier never binds), not this one's.
    if compact.contains(".bind(") {
        return;
    }
    scan_stmts(&block.stmts, source, const_names, findings);
}

fn scan_stmts(
    stmts: &[syn::Stmt],
    source: &str,
    const_names: &std::collections::HashSet<String>,
    findings: &mut Vec<SqlValueFinding>,
) {
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_expr(&init.expr, source, const_names, findings);
                }
            }
            syn::Stmt::Item(Item::Fn(f)) => {
                scan_stmts(&f.block.stmts, source, const_names, findings);
            }
            // Block descent FIRST (issue 227 D1): a query site inside an
            // if/match/block body is as reachable as a bare statement.
            syn::Stmt::Expr(inner, _) if block_shaped(inner) => {
                descend_block_expr(inner, source, const_names, findings);
            }
            syn::Stmt::Expr(e, _) => scan_expr(e, source, const_names, findings),
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

/// Descend a block-shaped expression's statement bodies (the value-slot
/// scan applies to the same statements a bare-expression walk would see).
fn descend_block_expr(
    e: &Expr,
    source: &str,
    const_names: &std::collections::HashSet<String>,
    findings: &mut Vec<SqlValueFinding>,
) {
    match e {
        Expr::If(eif) => {
            scan_stmts(&eif.then_branch.stmts, source, const_names, findings);
            if let Some((_, else_expr)) = &eif.else_branch {
                scan_expr(else_expr, source, const_names, findings);
            }
        }
        Expr::Match(m) => {
            for arm in &m.arms {
                scan_expr(&arm.body, source, const_names, findings);
            }
        }
        Expr::Block(b) => scan_stmts(&b.block.stmts, source, const_names, findings),
        Expr::Unsafe(u) => scan_stmts(&u.block.stmts, source, const_names, findings),
        Expr::Return(r) => {
            if let Some(ret) = &r.expr {
                scan_expr(ret, source, const_names, findings);
            }
        }
        _ => {}
    }
}

fn scan_expr(
    e: &Expr,
    source: &str,
    const_names: &std::collections::HashSet<String>,
    findings: &mut Vec<SqlValueFinding>,
) {
    match e {
        Expr::Macro(m) => {
            let name = m
                .mac
                .path
                .segments
                .last()
                .map_or(String::new(), |s| s.ident.to_string());
            if name == "format" || name == "write" {
                let lit = m.mac.tokens.to_string();
                if let Some(finding) = value_slot_finding(source, &lit, e, const_names) {
                    findings.push(finding);
                }
            }
        }
        Expr::Binary(op) if matches!(op.op, syn::BinOp::Add(_)) => {
            // The concat face: a string literal + a non-literal operand.
            let left = quote::ToTokens::to_token_stream(&op.left).to_string();
            let right_is_literal = matches!(&*op.right, Expr::Lit(_) | Expr::Const(_));
            if right_is_literal {
                return;
            }
            if let Some(finding) = value_slot_finding(source, &left, e, const_names) {
                findings.push(finding);
            }
        }
        _ => {}
    }
}

/// The value-slot gate: the literal carries SQL value-position vocabulary
/// (a `WHERE`/`VALUES`/`LIKE` keyword pair with a quoted placeholder, or
/// a bare `{}` adjacent to `= `), NOT an identifier position (ORDER BY /
/// GROUP BY adjacency is the OTHER rule's finding), and carries at least
/// one real placeholder (the source's `format_has_interpolation` law).
fn value_slot_finding(
    source: &str,
    lit_text: &str,
    e: &Expr,
    const_names: &std::collections::HashSet<String>,
) -> Option<SqlValueFinding> {
    let t = lit_text.to_string();
    // No placeholder → a constant string → the source's own FP law.
    if !has_placeholder(&t) {
        return None;
    }
    let has_sql_vocab = ["SELECT", "INSERT", "UPDATE", "DELETE", "WHERE"]
        .iter()
        .any(|k| t.to_uppercase().contains(k));
    if !has_sql_vocab {
        return None;
    }
    // Identifier-position refusal (the B350 rule owns that axis).
    let upper = t.to_uppercase();
    for kw in ["ORDER BY {", "GROUP BY {"] {
        if upper.contains(kw) {
            return None;
        }
    }
    // Value-position evidence: a quoted slot around a placeholder, or a
    // bare placeholder after `= ` / `VALUES (`.
    let value_slot = ["'{", "= {", "VALUES ({", "LIKE '{"]
        .iter()
        .any(|k| t.contains(k));
    if !value_slot {
        return None;
    }
    // The constant-binding law: every `{}`-slot operand a bare
    // identifier naming a file-level `const` is the static posture —
    // the interpolation cannot carry runtime data. Any dynamic operand
    // (or any slot whose operand is not a declared const ident) keeps
    // the finding.
    if slots_all_const(&t, const_names) {
        return None;
    }
    let byte_range = e.span().byte_range();
    Some(SqlValueFinding {
        site_span: byte_range.clone(),
        site_text: source.get(byte_range).unwrap_or("").to_string(),
        advice: ADVICE,
    })
}

/// Every `{name}` inline slot in the literal names a declared const AND
/// the format!'s positional arguments are all const idents (the implicit
/// `{}` face). A literal with no slots is handled by
/// `has_placeholder` upstream.
fn slots_all_const(lit_text: &str, const_names: &std::collections::HashSet<String>) -> bool {
    let mut inline_slots = Vec::new();
    let bytes = lit_text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if bytes.get(i + 1) == Some(&b'{') {
                i += 2;
                continue;
            }
            // Capture to the matching unescaped `}`.
            let mut j = i + 1;
            let mut inner = String::new();
            while j < bytes.len() && bytes[j] != b'}' {
                inner.push(bytes[j] as char);
                j += 1;
            }
            // The spec tail (`{:?}`, `{name:?}`) rides after `:` — the
            // operand name is the head.
            let head = inner.split(':').next().unwrap_or("").trim();
            inline_slots.push(head.to_string());
            i = j + 1;
            continue;
        }
        i += 1;
    }
    if inline_slots.is_empty() {
        // No inline slots → the implicit-positional face; the positional
        // args live in the macro text after the literal — every
        // comma-separated tail token must be a const ident. Unverifiable
        // text here is a NO (keep the finding).
        return false;
    }
    inline_slots.iter().all(|name| const_names.contains(name))
}

/// The source's `format_has_interpolation`: at least one real
/// interpolation placeholder; escaped braces (`{{`) do not count.
fn has_placeholder(macro_text: &str) -> bool {
    let bytes = macro_text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if bytes.get(i + 1) == Some(&b'{') {
                i += 2;
                continue;
            }
            return true;
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(src_text: &str) -> Vec<SqlValueFinding> {
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
            .find(|r| r.rule_id == "sql-query-value-interpolation-unbound")
            .expect("rule present");
        assert!(
            !findings(&rule.buggy_code).is_empty(),
            "the proposer must fire on its own buggy fixture"
        );
    }

    /// Batch 351 gate — the corpus `fixed_code` MUST stay silent (the
    /// fix carries `.bind(`, the fix's own suppression vocabulary).
    #[test]
    fn corpus_fixed_fixture_is_silent() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "sql-query-value-interpolation-unbound")
            .expect("rule present");
        assert!(
            findings(&rule.fixed_code).is_empty(),
            "the fixed fixture must stay silent"
        );
    }

    /// The quoted value slot with a dynamic value fires.
    #[test]
    fn value_slot_fires() {
        let src_text = "pub fn lookup(name: &str) -> Vec<Row> {\n    \
             let sql = format!(\"SELECT id FROM users WHERE name = '{name}'\");\n    \
             sqlx::query(&sql).fetch_all(&mut conn).unwrap()\n}\n";
        assert_eq!(findings(src_text).len(), 1, "the value slot fires");
    }

    /// The source's own FP law: no placeholder → silent.
    #[test]
    fn no_placeholder_silent() {
        let src_text = "pub fn ping() -> Vec<Row> {\n    \
             let sql = format!(\"SELECT 1\");\n    \
             sqlx::query(&sql).fetch_all(&mut conn).unwrap()\n}\n";
        assert!(findings(src_text).is_empty(), "static format! is silent");
    }

    /// The B350 anti-regime: an allowlisted-key identifier interpolation
    /// (`ORDER BY {key}`) is the OTHER rule's finding — this detector
    /// refuses the identifier position.
    #[test]
    fn identifier_position_refused() {
        let src_text = "pub fn list(sort_key: &str) -> Vec<Row> {\n    \
             let key = SORTABLE.contains(&sort_key).then_some(sort_key).unwrap_or(\"id\");\n    \
             let sql = format!(\"SELECT id FROM users ORDER BY {key}\");\n    \
             sqlx::query(&sql).fetch_all(&mut conn).unwrap()\n}\n";
        assert!(
            findings(src_text).is_empty(),
            "the ORDER BY position belongs to the identifier rule"
        );
    }

    /// The bind suppression (the fix's own vocabulary) — the buggy
    /// query shape PLUS `.bind(` goes silent (the suppression actually
    /// bites a real finding shape, not an empty fn).
    #[test]
    fn bind_suppresses() {
        let src_text = "pub fn lookup(name: &str) -> Vec<Row> {\n    \
             let sql = format!(\"SELECT id FROM users WHERE name = '{name}'\");\n    \
             sqlx::query(&sql).bind(name).fetch_all(&mut conn).unwrap()\n}\n";
        assert!(findings(src_text).is_empty(), "the .bind( shape suppresses");
    }

    /// The constant-binding law: every `{}` slot naming a file-level
    /// const is the static posture — silent.
    #[test]
    fn const_operands_silent() {
        let src_text = "const TRUSTED: &str = \"dashboard\";\n\npub fn q() -> Vec<Row> {\n    \
             let sql = format!(\"SELECT id FROM pages WHERE slug = '{TRUSTED}'\");\n    \
             sqlx::query(&sql).fetch_all(&mut conn).unwrap()\n}\n";
        assert!(findings(src_text).is_empty(), "all-const slots are silent");
    }

    /// One dynamic operand beside a const keeps the finding (the
    /// suppression requires EVERY slot to be const).
    #[test]
    fn mixed_const_dynamic_fires() {
        let src_text = "const TRUSTED: &str = \"dashboard\";\n\npub fn q(name: &str) -> Vec<Row> {\n    \
             let sql = format!(\"SELECT id FROM pages WHERE slug = '{TRUSTED}' AND owner = '{name}'\");\n    \
             sqlx::query(&sql).fetch_all(&mut conn).unwrap()\n}\n";
        assert_eq!(
            findings(src_text).len(),
            1,
            "a dynamic slot keeps the finding"
        );
    }

    /// The block-descent reachability: a site inside an `if` body fires.
    #[test]
    fn site_inside_if_block_fires() {
        let src_text = "pub fn lookup(name: &str, verbose: bool) -> Vec<Row> {\n    \
             if verbose {\n        \
             let sql = format!(\"SELECT id FROM users WHERE name = '{name}'\");\n        \
             sqlx::query(&sql).fetch_all(&mut conn).unwrap();\n    }\n    Vec::new()\n}\n";
        assert_eq!(
            findings(src_text).len(),
            1,
            "a query site inside an if block fires"
        );
    }

    /// No SQL sink in the fn → silent (the sink gate).
    #[test]
    fn no_sql_sink_silent() {
        let src_text = "pub fn greet(name: &str) -> String {\n    \
             format!(\"hello {name}'s tab\")\n}\n";
        assert!(findings(src_text).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src_text = "#[cfg(test)]\nmod tests {\n    \
             fn t(name: &str) -> Vec<Row> {\n        \
             let sql = format!(\"SELECT id FROM users WHERE name = '{name}'\");\n        \
             sqlx::query(&sql).fetch_all(&mut conn).unwrap()\n    }\n}\n";
        assert!(findings(src_text).is_empty());
    }
}

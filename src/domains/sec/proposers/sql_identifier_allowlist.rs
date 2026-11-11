//! `sql-identifier-interpolation-unallowlisted` — SUGGEST-ONLY detector
//! (Batch 350, the 0sec-labs/0 distill; Research 388 / Plan 356).
//!
//! A caller-chosen IDENTIFIER (a sort key, column name, group-by field)
//! interpolated into a SQL string: `format!("SELECT {col} FROM t ...")`,
//! `format!("ORDER BY {}", key)`. The injection surface is the KEY, not a
//! value — values are usually parameterised (`.bind(..)`), and identifiers
//! almost never are ("you can't bind an identifier" — the source's own
//! two-regime law), so the value-parameterization habit leaves the
//! identifier position wide open: a crafted key reaches the SQL parser
//! and becomes executable SQL — the structural-SQLI CWE-89 identifier face.
//!
//! ## The source anchor (0sec-labs/0 @ `f7020eb2`)
//!
//! `packages/core/src/agent/skills/vulnerabilities/structural-sqli.yaml`,
//! verbatim at the pin: "Values are usually parameterised, so they find
//! nothing. The high-value miss is when the backend concatenates a JSON
//! **key / field name** into SQL — these are almost never parameterised
//! because you can't bind an identifier." The surfaces it names:
//! `ORDER BY ${key}` / `GROUP BY ${key}` (sort/pagination params),
//! `SELECT ${key} FROM ...` (caller-chosen projection), and dynamic
//! `WHERE ${key} = ?` filter builders. This detector transcribes the
//! FIX half of that law (the audit half is attack-procedure content and
//! is never a gate).
//!
//! ## Why suggest-only (never a fix)
//!
//! The allowlist's CONTENT is the schema's answer (which columns are
//! sortable/projection-legal is the application's data model); the repair
//! shape (a const allowlist + `contains`/match gate before interpolation,
//! or a const column enum → literal) is bounded, but building it
//! mechanically would have to invent the column set. NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm.
//!
//! ## The bounded form
//!
//! ```text
//! const SORTABLE: [&str; 2] = ["name", "created_at"];
//! if !SORTABLE.contains(&sort_key) { return Err(..) } // the gate
//! let sql = format!("ORDER BY {sort_key}");           // now a checked key
//! ```
//!
//! ## Strict gates (ALL must hold; refuse more than fires)
//!
//! - **SQL-literal gate**: a string LITERAL carrying SQL
//!   identifier-position vocabulary — an `ORDER BY` / `GROUP BY`
//!   keyword pair, or a `SELECT`/`WHERE` keyword — with a format
//!   placeholder (`{}`, `{name}`) ADJACENT TO the keyword (the
//!   placeholder sits in the identifier position, not inside a quoted
//!   value slot in the literal).
//! - **Interpolation gate**: the literal is the format string of a
//!   `format!`/`write!`-family macro, or a `+` concat with a
//!   non-literal operand — the caller-chosen key actually flows.
//! - **Suppression, fn-wide (the fix's own vocabulary)**: any
//!   allowlist marker in the enclosing fn — `allowlist`, `whitelist`,
//!   an `ALLOWED_*`/`SORTABLE_*`/`VALID_*` const-style ident, or a
//!   `contains(`-gated key check — silences the fn (the key is checked
//!   before it interpolates; a deliberate dynamic-SQL design with a
//!   documented gate is the fixed posture).
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are
//!   skipped; macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - SQL assembled across helpers (one fn builds the fragment, another
//!   interpolates) is unconnected parse-only — fn-level gates only.
//! - Query builders (sea-query/diesel typed APIs) never carry a SQL
//!   string literal — an honest NO by construction (typed SQL is the
//!   fix by construction, the 0sec deserialize-DATA drop's shape).
//! - A `sqlx::query(&sql).bind(..)` chain does NOT suppress: values
//!   binding is the OTHER regime ("values bind, identifiers can't") —
//!   the interpolated identifier position is exactly what bind cannot
//!   cover. The two-regime note is carried in-rule, never as a gate.
//! - The literal-placeholder adjacency is a heuristic: a placeholder in
//!   the literal's VALUE slot (`WHERE name = '{name}'`) with a quoted
//!   slot fires too and review decides (the rule flags the pattern
//!   class, not a proof of reachability).
//! - Adjacent-DISTINCT from `unchecked-wire-arithmetic` IN-RULE: that
//!   rule owns the arithmetic-overflow sink (a wrapped count corrupts a
//!   length/key); THIS owns the string-interpolation sink into the SQL
//!   parser — different operator set (`BitAnd`/`+`-on-ints vs
//!   format!/concat), different sink (an allocator/wire field vs the
//!   database's parser).
//! - Batch 350 / Plan 356 / Research 388 (0sec-labs/0 @ `f7020eb2`,
//!   MIT OR Apache-2.0 — cited as class provenance; the fixture is an
//!   original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item};

/// One detected unallowlisted SQL identifier-interpolation site.
#[derive(Debug, Clone)]
pub struct SqlIdentifierFinding {
    /// Byte range of the interpolating expression.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the allowlist + the two-regime law).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a caller-chosen identifier interpolated into SQL \
is the structural SQLi surface — values bind (`{ .bind(..) }`), identifiers \
cannot, so `ORDER BY {key}`-shaped interpolation reaches the SQL parser \
verbatim. Gate the key against a const allowlist (`SORTABLE.contains(&key)`) \
or a const column enum before it interpolates (Batch 350, suggest-only)";

/// Detect unallowlisted SQL identifier interpolation (suggest-only; no
/// edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<SqlIdentifierFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<SqlIdentifierFinding>) {
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
            // fixture habitat, not production query assembly.
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

#[derive(Default)]
struct FnScan {
    sql_identifier_slot: Option<Range<usize>>,
    interpolated: bool,
    allowlisted: bool,
}

fn scan_fn(source: &str, block: &syn::Block, findings: &mut Vec<SqlIdentifierFinding>) {
    let mut walk = FnScan::default();
    scan_stmts(&block.stmts, source, &mut walk);
    // The fn-wide allowlist marker suppresses (the fix's own vocabulary):
    // a gated key is the fixed posture, whatever else the fn does.
    if walk.allowlisted {
        return;
    }
    if walk.interpolated {
        if let Some(span) = walk.sql_identifier_slot {
            findings.push(SqlIdentifierFinding {
                site_text: source[span.clone()].to_string(),
                site_span: span,
                advice: ADVICE,
            });
        }
    }
}

fn scan_stmts(stmts: &[syn::Stmt], source: &str, walk: &mut FnScan) {
    for stmt in stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    walk_expr(&init.expr, source, walk);
                }
            }
            syn::Stmt::Item(item) => collect_item_stmts(item, source, walk),
            syn::Stmt::Expr(e, _) => walk_expr(e, source, walk),
            syn::Stmt::Macro(_) => {}
        }
    }
}

fn collect_item_stmts(item: &Item, source: &str, walk: &mut FnScan) {
    match item {
        Item::Fn(f) => scan_stmts(&f.block.stmts, source, walk),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_stmts(&m.block.stmts, source, walk);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_item_stmts(nested, source, walk);
                }
            }
        }
        _ => {}
    }
}

fn walk_expr(e: &Expr, source: &str, walk: &mut FnScan) {
    match e {
        // `format!(".. {key} ..")` — the interpolation gate.
        Expr::Macro(m) => {
            let seg = m.mac.path.segments.last().map(|s| s.ident.to_string());
            if matches!(
                seg.as_deref(),
                Some("format" | "write" | "writeln" | "panic")
            ) {
                walk.interpolated = true;
                for lit in format_string_literals(&m.mac.tokens.to_token_stream().to_string()) {
                    if walk.sql_identifier_slot.is_none() && literal_has_identifier_slot(&lit) {
                        let span = e.span().byte_range();
                        if span.end <= source.len() {
                            walk.sql_identifier_slot = Some(span);
                        }
                    }
                }
            }
            // Macro tokens are never descended into (house recall limit).
        }
        // `"...".to_owned() + key` / `String::from("..") + &key` — the
        // concat face of the interpolation gate. The SQL-slot check reads
        // the LEFT side's literals off its rendered tokens (the literal
        // may sit inside a `String::from(..)`/`to_owned()` call, not as a
        // bare literal); a literal with no placeholder fires via the
        // ends-in-keyword face (the `+` IS the interpolation there).
        Expr::Binary(b) if matches!(b.op, syn::BinOp::Add(_)) => {
            let left_ts = quote::ToTokens::to_token_stream(b.left.as_ref()).to_string();
            let right_is_variable = !is_const_operand(&b.right);
            for lit in format_string_literals(&left_ts) {
                let slot = literal_has_identifier_slot(&lit)
                    || (right_is_variable && literal_ends_in_identifier_slot(&lit));
                if walk.sql_identifier_slot.is_none() && slot {
                    let span = b.left.span().byte_range();
                    if span.end <= source.len() {
                        walk.sql_identifier_slot = Some(span);
                    }
                }
            }
            walk.interpolated |= right_is_variable;
            walk_expr(&b.left, source, walk);
            walk_expr(&b.right, source, walk);
        }
        Expr::MethodCall(m) => {
            // The suppression face: `.contains(&key)` over an allowlist
            // const, or the key routed through a match — any allowlist
            // marker silences the fn.
            if m.method == "contains" || m.method == "iter" {
                walk.allowlisted = true;
            }
            walk_expr(&m.receiver, source, walk);
            for a in &m.args {
                walk_expr(a, source, walk);
            }
        }
        Expr::Call(c) => {
            walk_expr(&c.func, source, walk);
            for a in &c.args {
                walk_expr(a, source, walk);
            }
        }
        _ => scan_child_exprs(e, source, walk),
    }
}

fn scan_child_exprs(e: &Expr, source: &str, walk: &mut FnScan) {
    match e {
        Expr::If(i) => {
            walk_expr(&i.cond, source, walk);
            scan_stmts(&i.then_branch.stmts, source, walk);
            if let Some((_, else_e)) = &i.else_branch {
                walk_expr(else_e, source, walk);
            }
        }
        Expr::Unary(u) => walk_expr(&u.expr, source, walk),
        Expr::Paren(p) => walk_expr(&p.expr, source, walk),
        Expr::Reference(r) => walk_expr(&r.expr, source, walk),
        Expr::Match(m) => {
            walk_expr(&m.expr, source, walk);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    walk_expr(guard, source, walk);
                }
                walk_expr(&arm.body, source, walk);
            }
        }
        Expr::Block(b) => scan_stmts(&b.block.stmts, source, walk),
        Expr::Unsafe(u) => scan_stmts(&u.block.stmts, source, walk),
        Expr::Loop(l) => scan_stmts(&l.body.stmts, source, walk),
        Expr::While(w) => {
            walk_expr(&w.cond, source, walk);
            scan_stmts(&w.body.stmts, source, walk);
        }
        Expr::ForLoop(fl) => {
            walk_expr(&fl.expr, source, walk);
            scan_stmts(&fl.body.stmts, source, walk);
        }
        Expr::Try(t) => walk_expr(&t.expr, source, walk),
        _ => {}
    }
}

/// The fn-wide allowlist markers (the fix's own vocabulary), read off the
/// raw token text — an `ALLOWED_*`/`SORTABLE_*` const, a `whitelist`
/// spelling, or a match-gate on the key.
/// The SQL identifier-position vocabulary + a format placeholder ADJACENT
/// to it (the placeholder sits in the identifier slot, not inside a
/// quoted value slot). The adjacency is token-level: `order by {}`,
/// `order by {key}`, `group by {k}`, `select {cols} from`, and the
/// `where {k} =` filter-builder face.
fn literal_has_identifier_slot(lit: &str) -> bool {
    let lower = lit.to_ascii_lowercase();
    let has_placeholder = lower.contains("{}") || (lower.contains('{') && lower.contains('}'));
    if !has_placeholder {
        return false;
    }
    let keyword_slot = [
        ("order by", 0),
        ("group by", 0),
        ("select ", 1),
        ("where ", 2),
    ];
    for (kw, mode) in keyword_slot {
        if let Some(pos) = lower.find(kw) {
            let after = &lower[pos + kw.len()..];
            let placeholder_ahead = after.find('{');
            // mode 0: the placeholder must come RIGHT after the keyword
            // (optionally past a comma/expression the builder owns).
            // mode 1: `select {..} from` — placeholder before FROM.
            // mode 2: `where {..} =` — the dynamic filter-builder face.
            let ok = match mode {
                0 => placeholder_ahead.is_some_and(|p| p < 3) || after.starts_with('{'),
                1 => {
                    placeholder_ahead.is_some()
                        && after[placeholder_ahead.unwrap()..].contains("from")
                }
                _ => placeholder_ahead
                    .is_some_and(|p| after[p..].contains('=') || after.contains(" in (")),
            };
            if ok {
                return true;
            }
        }
    }
    false
}

/// The concat-face slot: the literal ENDS in identifier-position
/// vocabulary (`.. ORDER BY `, `.. GROUP BY`, `.. SELECT`, `.. WHERE`) —
/// the `+` provides the identifier that follows.
fn literal_ends_in_identifier_slot(lit: &str) -> bool {
    let lower = lit.to_ascii_lowercase();
    let trimmed = lower.trim_end();
    ["order by", "group by", "select", "where"]
        .iter()
        .any(|kw| trimmed.ends_with(kw))
}

/// A constant operand — a literal, or a path that cannot name a plain
/// local: multi-segment (`std::u8::MAX`, `Self::DEFAULT`) or an
/// uppercase-first single segment (`DEFAULT_ORDER`, a const by the
/// naming convention). A single lowercase-first segment (`col`,
/// `sort_key`) is a plain variable — the caller-fed face.
fn is_const_operand(e: &Expr) -> bool {
    match e {
        Expr::Lit(_) => true,
        Expr::Path(p) => {
            if p.path.segments.len() > 1 {
                return true;
            }
            let seg = p.path.segments.last().map(|s| s.ident.to_string());
            seg.is_some_and(|s| s.starts_with(|c: char| c.is_ascii_uppercase()))
        }
        _ => false,
    }
}

fn format_string_literals(tokens: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = tokens;
    while let Some(start) = rest.find('"') {
        let after = &rest[start + 1..];
        if let Some(end) = after.find('"') {
            out.push(after[..end].to_string());
            rest = &after[end + 1..];
        } else {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::SqlIdentifierFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 350 gate — the corpus `buggy_code` MUST fire.
    #[test]
    fn batch350_order_by_interpolation_fires() {
        let src = "pub fn list(sort_key: &str) -> String {\n    \
             format!(\"SELECT id, name FROM users ORDER BY {sort_key}\")\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the identifier slot fires: {f:?}");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// The fixed posture — the key gated through a const allowlist — is
    /// silent (fn-wide suppression).
    #[test]
    fn allowlisted_key_is_silent() {
        let src = "const SORTABLE: [&str; 2] = [\"name\", \"created_at\"];\n\n\
             pub fn list(sort_key: &str) -> String {\n    \
             if !SORTABLE.contains(&sort_key) {\n        \
             return String::from(\"ORDER BY id\");\n    }\n    \
             format!(\"SELECT id FROM users ORDER BY {sort_key}\")\n}\n";
        assert!(findings(src).is_empty(), "the allowlist suppresses fn-wide");
    }

    /// A fully-static SQL string (no interpolation in the fn) is silent.
    #[test]
    fn static_sql_is_silent() {
        let src = "pub fn list() -> String {\n    \
             String::from(\"SELECT id, name FROM users ORDER BY name\")\n}\n";
        assert!(findings(src).is_empty(), "no interpolation, no fire");
    }

    /// Value-parameter interpolation (`WHERE name = '{v}'`) with no
    /// identifier-slot adjacency does NOT fire the slot gate — but a
    /// `where {k} =` filter-builder face does.
    #[test]
    fn value_slot_is_silent_filter_builder_fires() {
        let value_slot = "pub fn find(name: &str) -> String {\n    \
             format!(\"SELECT id FROM users WHERE name = '{name}'\")\n}\n";
        assert!(
            findings(value_slot).is_empty(),
            "a quoted value slot is the other regime (values bind)"
        );
        let filter_builder = "pub fn filtered(col: &str, v: &str) -> String {\n    \
             format!(\"SELECT id FROM t WHERE {col} = '{v}'\")\n}\n";
        assert_eq!(
            findings(filter_builder).len(),
            1,
            "the dynamic filter-builder face fires"
        );
    }

    /// A `bind()` chain does NOT suppress — values binding cannot cover
    /// the identifier position (the source's two-regime law, in-rule).
    #[test]
    fn bind_chain_does_not_suppress() {
        let src = "pub fn page(conn: &mut Conn, sort_key: &str, limit: i64) -> Vec<Row> {\n    \
             let sql = format!(\"SELECT id FROM t ORDER BY {sort_key} LIMIT ?\");\n    \
             sqlx::query_as::<_, Row>(&sql).bind(limit).fetch_all(conn).unwrap()\n}\n";
        assert_eq!(
            findings(src).len(),
            1,
            "bind covers values, not identifiers"
        );
    }

    /// The concat face fires.
    #[test]
    fn concat_face_fires() {
        let src = "pub fn group(col: &str) -> String {\n    \
             String::from(\"SELECT dept, count(*) FROM emp GROUP BY \") + col\n}\n";
        assert_eq!(findings(src).len(), 1, "the concat face fires");
    }

    /// The concat face with a CONST right operand is silent (nothing
    /// caller-fed interpolates).
    #[test]
    fn concat_const_operand_silent() {
        let src = "pub fn base() -> String {\n    \
             String::from(\"SELECT id FROM t ORDER BY \") + DEFAULT_ORDER\n}\n";
        assert!(findings(src).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             fn t(sort_key: &str) -> String {\n        \
             format!(\"SELECT id FROM users ORDER BY {sort_key}\")\n    }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Batch 350 gate — the corpus `fixed_code` MUST stay silent.
    #[test]
    fn corpus_fixed_fixture_is_silent() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "sql-identifier-interpolation-unallowlisted")
            .expect("rule present");
        let fixed = syn::parse_file(&rule.fixed_code).expect("fixed parses");
        assert!(
            detect(&rule.fixed_code, &fixed).is_empty(),
            "the fixed fixture must stay silent"
        );
    }
}

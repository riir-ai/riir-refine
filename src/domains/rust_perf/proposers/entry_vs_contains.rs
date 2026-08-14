//! `entry-vs-contains-insert` — `contains_key()` + `insert()` → `entry().or_insert()`.
//!
//! Pattern: the *statement* `if !map.contains_key(&k) { map.insert(k, v); }`
//! (single-stmt then-block, no else, no attributes) → the statement
//! `map.entry(k).or_insert(v);`.
//!
//! The rewrite merges two hash lookups (contains_key hashes + probes, then
//! insert hashes + probes again) into one. Bench 013: ParetoDominant
//! 1.5-2.4x. The leading `&` on the key is stripped (`contains_key(&k)`
//! takes `&K`; `entry(k)` takes `K` by value — the corpus form).
//!
//! Rewriting at the **statement** level is load-bearing: the original
//! if-statement produces `()`, and so does a semicolon'd
//! `map.entry(k).or_insert(v);` — but the bare method call in value
//! position would be `&mut V`, and without the semi it would not even
//! re-parse outside tail position.
//!
//! Span-preserving: the whole if-statement span is replaced with
//! `<recv>.entry(<key text>).or_insert(<val text>);` where every fragment is
//! the ORIGINAL source text of the matched nodes.
//!
//! Out of scope (documented, not silently mishandled): the positive form
//! `if map.contains_key(&k) {} else { .. }`, multi-stmt then-blocks,
//! attribute-carrying ifs, and non-path receivers (`!self.map.contains_key(..)`
//! parses as `Expr::Field` — skipped).

use quote::ToTokens;
use syn::visit::Visit;
use syn::{Expr, ExprIf, File, Stmt, UnOp};

use super::{edit, src, SourceEdit};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    // The statement can nest anywhere (match-arm blocks, closures, loop
    // bodies, ...), so walk every stmt in the file.
    let mut scan = IfScan { source, edits: Vec::new() };
    scan.visit_file(file);
    scan.edits
}

struct IfScan<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for IfScan<'_> {
    fn visit_stmt(&mut self, s: &'ast Stmt) {
        if let Stmt::Expr(Expr::If(eif), _) = s {
            if let Some(e) = self.rewrite_if_stmt(eif) {
                self.edits.push(e);
            }
        }
        syn::visit::visit_stmt(self, s);
    }
}

impl IfScan<'_> {
    /// Match + rewrite one `if !m.contains_key(&k) { m.insert(k, v); }` into
    /// the edit replacing it with `m.entry(k).or_insert(v);` — every
    /// fragment is the ORIGINAL source text of the matched node.
    fn rewrite_if_stmt(&self, eif: &ExprIf) -> Option<SourceEdit> {
        if !eif.attrs.is_empty() {
            return None;
        }

        // cond: !recv.contains_key(<key>)
        let Expr::Unary(unary) = &*eif.cond else { return None };
        let UnOp::Not(_) = unary.op else { return None };
        let Expr::MethodCall(ck) = &*unary.expr else { return None };
        if ck.method != "contains_key" || ck.args.len() != 1 {
            return None;
        }
        let Expr::Path(recv) = &*ck.receiver else { return None };

        // then: exactly one stmt — recv.insert(k, v)
        if eif.else_branch.is_some() || eif.then_branch.stmts.len() != 1 {
            return None;
        }
        let Stmt::Expr(Expr::MethodCall(ins), _) = &eif.then_branch.stmts[0] else {
            return None;
        };
        if ins.method != "insert" || ins.args.len() != 2 {
            return None;
        }
        let Expr::Path(ins_recv) = &*ins.receiver else { return None };
        if ins_recv.path.to_token_stream().to_string() != recv.path.to_token_stream().to_string() {
            return None;
        }

        let key = ck.args.first()?;
        let val = ins.args.last()?;

        // `contains_key(&k)` → `entry(k)`: the INNER (un-referenced) node's
        // original text; a bare key node is used as-is.
        let key_node: &Expr = match key {
            Expr::Reference(eref) => &eref.expr,
            other => other,
        };
        let replacement = format!(
            "{}.entry({}).or_insert({});",
            src(self.source, recv)?,
            src(self.source, key_node)?,
            src(self.source, val)?
        );
        Some(edit(eif, replacement))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn corpus_fixture_rewrites() {
        let out = propose_fix(
            "entry-vs-contains-insert",
            "use std::collections::HashMap;\nfn insert_if_absent(map: &mut \
             HashMap<u32, u32>, k: u32, v: u32) {\n    if !map.contains_key(&k) {\n        \
             map.insert(k, v);\n    }\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("map.entry(k).or_insert(v);"), "{out}");
        assert!(!out.contains("contains_key"), "{out}");
        // The fn signature line is byte-identical (span preservation).
        assert!(
            out.contains("fn insert_if_absent(map: &mut HashMap<u32, u32>, k: u32, v: u32) {"),
            "{out}"
        );
    }

    #[test]
    fn mid_block_statement_rewrites_with_semi() {
        // The load-bearing case: the if-stmt is NOT in tail position. The
        // replacement must carry its own semicolon to re-parse.
        let out = propose_fix(
            "entry-vs-contains-insert",
            "fn f(map: &mut std::collections::HashMap<u32, u32>, k: u32) -> u32 {\n    \
             if !map.contains_key(&k) { map.insert(k, 1); }\n    k + 1\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("map.entry(k).or_insert(1);"), "{out}");
        let reparsed = syn::parse_file(&out).expect("output must re-parse");
        assert!(!reparsed.items.is_empty());
    }

    #[test]
    fn nested_in_match_arm_rewrites() {
        let out = propose_fix(
            "entry-vs-contains-insert",
            "fn f(map: &mut std::collections::HashMap<u32, u32>, x: u32) {\n    match x {\n        \
             0 => { if !map.contains_key(&x) { map.insert(x, x); } }\n        _ => {}\n    }\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("map.entry(x).or_insert(x);"), "{out}");
    }

    #[test]
    fn already_present_ref_key_stays() {
        // Key passed without a leading & (already a reference): entry(k) as-is.
        let out = propose_fix(
            "entry-vs-contains-insert",
            "fn f(map: &mut std::collections::HashMap<String, u32>, k: &String) {\n    \
             if !map.contains_key(k) { map.insert(k.clone(), 1); }\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("map.entry(k).or_insert(1);"), "{out}");
    }

    #[test]
    fn complex_value_expression_preserved() {
        let out = propose_fix(
            "entry-vs-contains-insert",
            "fn f(map: &mut std::collections::HashMap<u32, u32>, k: u32) {\n    \
             if !map.contains_key(&k) { map.insert(k, k * 2 + compute(k)); }\n}\nfn compute(x: u32) -> u32 { x }",
        )
        .expect("pattern matched");
        assert!(out.contains("map.entry(k).or_insert(k * 2 + compute(k));"), "{out}");
    }

    #[test]
    fn multi_stmt_then_unchanged() {
        assert!(propose_fix(
            "entry-vs-contains-insert",
            "fn f(map: &mut std::collections::HashMap<u32, u32>, k: u32) {\n    \
             if !map.contains_key(&k) { map.insert(k, 1); map.insert(k + 1, 2); }\n}"
        )
        .is_none());
    }

    #[test]
    fn with_else_unchanged() {
        assert!(propose_fix(
            "entry-vs-contains-insert",
            "fn f(map: &mut std::collections::HashMap<u32, u32>, k: u32) -> u32 {\n    \
             if !map.contains_key(&k) { map.insert(k, 1); } else { return 2; }\n    0\n}"
        )
        .is_none());
    }

    #[test]
    fn positive_form_unchanged() {
        assert!(propose_fix(
            "entry-vs-contains-insert",
            "fn f(map: &mut std::collections::HashMap<u32, u32>, k: u32) {\n    \
             if map.contains_key(&k) { map.insert(k, 1); }\n}"
        )
        .is_none());
    }

    #[test]
    fn mismatched_receivers_unchanged() {
        assert!(propose_fix(
            "entry-vs-contains-insert",
            "fn f(a: &mut std::collections::HashMap<u32, u32>, b: &mut \
             std::collections::HashMap<u32, u32>, k: u32) {\n    \
             if !a.contains_key(&k) { b.insert(k, 1); }\n}"
        )
        .is_none());
    }

    #[test]
    fn attributed_if_unchanged() {
        assert!(propose_fix(
            "entry-vs-contains-insert",
            "#[allow(unused)]\nfn f(map: &mut std::collections::HashMap<u32, u32>, k: u32) {\n    \
             #[allow(clippy::all)]\n    if !map.contains_key(&k) { map.insert(k, 1); }\n}"
        )
        .is_none());
    }
}

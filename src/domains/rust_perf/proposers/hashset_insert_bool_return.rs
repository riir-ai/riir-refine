//! `hashset-insert-bool-return` — `!set.contains(k)` guard + `set.insert(k)`
//! body → the insert call's own bool return.
//!
//! Pattern: the statement
//! `if !recv.contains(ARG) { ..; recv.insert(KEY); .. }` (single matching
//! `insert`, same receiver, key equal modulo leading `&`/`*` layers) →
//! the condition becomes `recv.insert(KEY)` and the insert statement is
//! deleted; every other then-branch statement survives verbatim.
//!
//! `HashSet::insert` returns `true` exactly when the value was newly
//! inserted — i.e. when `!contains` held — so the branch taken is
//! identical while the double lookup (contains hashes + probes, insert
//! hashes + probes again) collapses to one. Bench 020/B48: ParetoDominant
//! 1.19-1.31x. `BTreeSet::insert` has the same bool contract (the match is
//! shape-based; any receiver type whose 1-arg `insert` returns bool works).
//!
//! The KEY text in the new condition is the INSERT call's own argument
//! (verbatim) — it is the source of truth for what gets inserted. The
//! contains-arg/insert-arg equivalence gate is load-bearing: a guard
//! checking `a` while inserting `b` must NOT be merged (the rewritten
//! condition would check `b` instead). Leading `&`/`*` layers are stripped
//! on both sides before comparing (`contains(zone)` + `insert(*zone)` is
//! the canonical iteration-over-`&T` form).
//!
//! Span-preserving: two edits — the `!recv.contains(..)` condition span →
//! `recv.insert(KEY)`, and a whole-statement deletion of the insert (via
//! [`delete_stmt_edit`], which removes the line's scaffolding when the
//! insert owns its line).
//!
//! Out of scope (documented, not silently mishandled): the positive guard
//! form `if recv.contains(k) { .. } else { insert }`, guards with extra
//! conjuncts (`!contains(..) && other`), `let _ = recv.insert(k);` forms,
//! multi-arg inserts (a `Vec::insert(idx, val)` is skipped by the 1-arg
//! gate), and non-path receivers (`!self.seen.contains(..)` parses as
//! `Expr::Field` — skipped).

use quote::ToTokens;
use syn::visit::Visit;
use syn::{Expr, ExprIf, ExprMethodCall, File, Stmt, UnOp};

use super::{delete_stmt_edit, edit, src, SourceEdit};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    // The if-statement can nest anywhere (loop bodies, match arms, closures),
    // so walk every stmt in the file.
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
            self.rewrite_if_stmt(eif);
        }
        syn::visit::visit_stmt(self, s);
    }
}

impl IfScan<'_> {
    /// Match + rewrite one `if !recv.contains(arg) { .. recv.insert(key) .. }`.
    /// Pushes the condition replacement + the insert-stmt deletion.
    fn rewrite_if_stmt(&mut self, eif: &ExprIf) {
        if !eif.attrs.is_empty() {
            return;
        }

        // cond: !recv.contains(arg)
        let Expr::Unary(unary) = &*eif.cond else { return };
        let UnOp::Not(_) = unary.op else { return };
        let Expr::MethodCall(ck) = &*unary.expr else { return };
        if ck.method != "contains" || ck.args.len() != 1 {
            return;
        }
        let Expr::Path(recv) = &*ck.receiver else { return };
        let Some(contains_arg) = ck.args.first() else { return };

        // then: exactly one `recv.insert(key)` stmt with an equivalent key.
        let recv_txt = match recv.path.to_token_stream().to_string() {
            t if !t.is_empty() => t,
            _ => return,
        };
        let contains_key = tokens(stripped(contains_arg));
        let mut matched: Option<(&Stmt, &ExprMethodCall)> = None;
        let mut match_count = 0usize;
        for stmt in &eif.then_branch.stmts {
            let Stmt::Expr(Expr::MethodCall(ins), _) = stmt else { continue };
            if ins.method != "insert" || ins.args.len() != 1 {
                continue;
            }
            let Expr::Path(ins_recv) = &*ins.receiver else { continue };
            let Some(ins_arg) = ins.args.first() else { continue };
            if ins_recv.path.to_token_stream().to_string() != recv_txt {
                continue;
            }
            if tokens(stripped(ins_arg)) != contains_key {
                continue;
            }
            match_count += 1;
            matched = Some((stmt, ins));
        }
        if match_count != 1 {
            return;
        }
        let Some((ins_stmt, ins)) = matched else { return };

        // New condition text from the INSERT call's own fragments.
        let Some(recv_src) = src(self.source, recv) else { return };
        let Some(key_src) = src(self.source, ins.args.first().expect("1 arg")) else {
            return;
        };
        let new_cond = format!("{recv_src}.insert({key_src})");

        self.edits.push(edit(&eif.cond, new_cond));
        self.edits.push(delete_stmt_edit(self.source, ins_stmt));
    }
}

/// Strip leading `&`/`*` unary layers (reference/deref are transparent for
/// the key-equivalence check: `zone`, `&zone`, `*zone`, `&&zone` all check
/// the same value when one side is a plain path binding).
fn stripped(mut expr: &Expr) -> &Expr {
    loop {
        match expr {
            Expr::Reference(r) => expr = &r.expr,
            Expr::Unary(u) if matches!(u.op, UnOp::Deref(_)) => expr = &u.expr,
            _ => return expr,
        }
    }
}

fn tokens(expr: &Expr) -> String {
    expr.to_token_stream().to_string()
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn corpus_fixture_rewrites() {
        let out = propose_fix(
            "hashset-insert-bool-return",
            "use std::collections::HashSet;\nfn register_seen(seen: &mut \
             HashSet<u32>, items: &[u32]) -> usize {\n    let mut inserted = 0;\n    \
             for &x in items {\n        if !seen.contains(&x) {\n            \
             seen.insert(x);\n            inserted += 1;\n        }\n    }\n    \
             inserted\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("if seen.insert(x) {"), "{out}");
        assert!(!out.contains("contains"), "{out}");
        // The insert stmt is gone; the counter survives.
        assert!(!out.contains("seen.insert(x);"), "{out}");
        assert!(out.contains("inserted += 1;"), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    /// The eval-span shape: iterating `&T` gives `contains(zone)` (no `&`)
    /// paired with `insert(*zone)` (deref) — equal after layer stripping.
    #[test]
    fn ref_deref_key_layers_match() {
        let out = propose_fix(
            "hashset-insert-bool-return",
            "use std::collections::HashSet;\n\npub fn track_visited_zones(visited: &mut \
             HashSet<ZoneId>, path: &[ZoneId]) -> usize {\n    let mut added = 0;\n    \
             for zone in path {\n        if !visited.contains(zone) {\n            \
             visited.insert(*zone);\n            added += 1;\n        }\n    }\n    added\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("if visited.insert(*zone) {"), "{out}");
        assert!(out.contains("added += 1;"), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn derefd_receiver_field_skipped() {
        // `self.seen` parses as Expr::Field — conservatively skipped.
        assert!(propose_fix(
            "hashset-insert-bool-return",
            "fn f(&self, k: u32) {\n    if !self.seen.contains(&k) {\n        \
             self.seen.insert(k);\n    }\n}",
        )
        .is_none());
    }

    #[test]
    fn key_mismatch_skipped() {
        // Guard checks `a` but inserts `b` — merging would change semantics.
        assert!(propose_fix(
            "hashset-insert-bool-return",
            "use std::collections::HashSet;\nfn f(s: &mut HashSet<u32>, a: u32, b: u32) {\n    \
             if !s.contains(&a) {\n        s.insert(b);\n    }\n}",
        )
        .is_none());
    }

    #[test]
    fn mismatched_receivers_skipped() {
        assert!(propose_fix(
            "hashset-insert-bool-return",
            "use std::collections::HashSet;\nfn f(a: &mut HashSet<u32>, b: &mut \
             HashSet<u32>, k: u32) {\n    if !a.contains(&k) {\n        b.insert(k);\n    }\n}",
        )
        .is_none());
    }

    #[test]
    fn two_arg_insert_skipped() {
        // Vec::insert(idx, val) shape — 2 args, different semantics.
        assert!(propose_fix(
            "hashset-insert-bool-return",
            "fn f(v: &mut Vec<u32>, k: usize) {\n    if !v.contains(&k) {\n        \
             v.insert(k, 0);\n    }\n}",
        )
        .is_none());
    }

    #[test]
    fn duplicate_matching_inserts_skipped() {
        assert!(propose_fix(
            "hashset-insert-bool-return",
            "use std::collections::HashSet;\nfn f(s: &mut HashSet<u32>, k: u32) {\n    \
             if !s.contains(&k) {\n        s.insert(k);\n        s.insert(k);\n    }\n}",
        )
        .is_none());
    }

    #[test]
    fn conjunct_guard_skipped() {
        assert!(propose_fix(
            "hashset-insert-bool-return",
            "use std::collections::HashSet;\nfn f(s: &mut HashSet<u32>, k: u32, ok: bool) {\n    \
             if !s.contains(&k) && ok {\n        s.insert(k);\n    }\n}",
        )
        .is_none());
    }

    #[test]
    fn positive_form_skipped() {
        assert!(propose_fix(
            "hashset-insert-bool-return",
            "use std::collections::HashSet;\nfn f(s: &mut HashSet<u32>, k: u32) {\n    \
             if s.contains(&k) {\n        s.insert(k);\n    }\n}",
        )
        .is_none());
    }

    #[test]
    fn else_branch_preserved() {
        // insert() == !contains() — the else arm still selects on the same
        // predicate, so the rewrite stays valid with an else present.
        let out = propose_fix(
            "hashset-insert-bool-return",
            "use std::collections::HashSet;\nfn f(s: &mut HashSet<u32>, k: u32) -> u32 {\n    \
             if !s.contains(&k) {\n        s.insert(k);\n        1\n    } else {\n        2\n    }\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("if s.insert(k) {"), "{out}");
        assert!(out.contains("} else {"), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn insert_only_stmt_keeps_empty_block() {
        // Degenerate but correct: the side effect moves into the condition
        // and the then-block empties (braces remain — span preservation) —
        // still parses, still inserts.
        let out = propose_fix(
            "hashset-insert-bool-return",
            "use std::collections::HashSet;\nfn f(s: &mut HashSet<u32>, k: u32) {\n    \
             if !s.contains(&k) {\n        s.insert(k);\n    }\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("if s.insert(k) {"), "{out}");
        assert!(!out.contains("insert(k);"), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }
}

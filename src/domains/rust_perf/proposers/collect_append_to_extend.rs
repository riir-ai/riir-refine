//! `collect-append-to-extend` — `collect()` into a staging `Vec` +
//! `append(&mut staged)` → `extend()` straight into the target.
//!
//! Pattern: within one block,
//! `let mut VAR [: TY] = <iter-expr>.collect();` followed (with no
//! intervening statement mentioning `VAR`) by `RECV.append(&mut VAR);`
//! → the `let` is deleted and the append becomes `RECV.extend(<iter-expr>)`.
//! The intermediate `Vec` allocation disappears entirely — Bench 020/B48:
//! ParetoDominant 8.5-12.8x, allocs 6251→1 at the bench shape.
//!
//! Gates (all load-bearing):
//! - **Single-use staging:** `VAR` must occur exactly twice in the whole
//!   file (the `let` + the append argument). A later read of `VAR`
//!   (`staged.len()` after the append — legal: `append` takes `&mut` and
//!   empties, leaving the binding alive) or a same-named binding in a
//!   sibling scope would break the rewrite, so both skip. This also skips
//!   occurrences in comments (conservative by design).
//! - **First-mention adjacency:** the first later statement in the block
//!   that mentions `VAR` at all must BE the append — any intervening use
//!   (`staged.push(..)`, a nested block reading it) skips the pattern.
//! - A later re-binding of `VAR` stops the scan (shadowing ambiguity,
//!   the vec-with-capacity scope-gate precedent).
//!
//! The rewrite is receiver-agnostic (`RECV` may be a field like
//! `self.buf`) and drops the collect's turbofish + the let's type
//! annotation: `append` requires the staging element type to equal the
//! target's, so `extend(<iter-expr>)` infers identically.
//!
//! Out of scope: `append` in a nested block (the `let` and the append must
//! share a block), `Vec::from_iter(..)` forms, non-`&mut` append args, and
//! multi-arg methods named `append`.

use syn::visit::Visit;
use syn::{Block, Expr, File, Local, Pat, Stmt};

use super::{delete_stmt_edit, edit, span_range, src, SourceEdit};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut scan = BlockScan { source, edits: Vec::new() };
    scan.visit_file(file);
    scan.edits
}

struct BlockScan<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for BlockScan<'_> {
    // fn bodies, loop bodies, match-arm blocks — the corpus/eval shapes live
    // inside a `for` body, so the scan must run at EVERY block, not just the
    // fn's top level.
    fn visit_block(&mut self, b: &'ast Block) {
        self.scan_block(b);
        syn::visit::visit_block(self, b);
    }
}

impl BlockScan<'_> {
    fn scan_block(&mut self, block: &Block) {
        let stmts = &block.stmts;
        for (i, stmt) in stmts.iter().enumerate() {
            let Stmt::Local(local) = stmt else { continue };
            let Some(var) = staged_binding(local) else { continue };

            // First later stmt mentioning VAR must be `RECV.append(&mut VAR);`.
            let mut append: Option<(&Stmt, &Expr)> = None;
            for later in stmts.iter().skip(i + 1) {
                if stmt_binds_ident(later, &var) {
                    break; // shadowing re-bind — stop the scan
                }
                if !self.stmt_mentions(later, &var) {
                    continue;
                }
                append = match_append(later, &var);
                break;
            }
            let Some((append_stmt, recv_expr)) = append else { continue };

            // Single-use staging: exactly the let + the append arg in the file.
            if count_ident_words(self.source, &var) != 2 {
                continue;
            }

            let Stmt::Local(local) = stmt else { continue };
            let Some(init) = &local.init else { continue };
            let Expr::MethodCall(collect) = &*init.expr else { continue };
            let Some(recv_txt) = src(self.source, recv_expr) else { continue };
            let Some(iter_txt) = src(self.source, &*collect.receiver) else { continue };

            let extend = format!("{recv_txt}.extend({iter_txt});");
            self.edits.push(delete_stmt_edit(self.source, stmt));
            self.edits.push(edit(append_stmt, extend));
        }
    }

    /// Word-boundary mention of `var` anywhere in the stmt's source text.
    fn stmt_mentions(&self, stmt: &Stmt, var: &str) -> bool {
        let (s, e) = span_range(stmt);
        match self.source.get(s..e) {
            Some(text) => contains_ident_word(text, var),
            // Defensive: if spans don't slice cleanly, treat as a mention
            // (conservative — skips the rewrite rather than risking one).
            None => true,
        }
    }
}

/// Is `later` exactly `RECV.append(&mut VAR);` (receiver: any expr)?
fn match_append<'a>(later: &'a Stmt, var: &str) -> Option<(&'a Stmt, &'a Expr)> {
    let Stmt::Expr(Expr::MethodCall(app), _) = later else { return None };
    if app.method != "append" || app.args.len() != 1 {
        return None;
    }
    let Expr::Reference(eref) = app.args.first()? else { return None };
    eref.mutability?;
    let Expr::Path(p) = &*eref.expr else { return None };
    if !p.path.is_ident(var) {
        return None;
    }
    Some((later, &app.receiver))
}

/// `let mut VAR [: TY] = <expr>.collect();` → the staging ident.
/// Mirrors vec-with-capacity's `fresh_vec_binding` gates: `mut` binding,
/// no `ref`/subpattern, no `let else`.
fn staged_binding(local: &Local) -> Option<String> {
    // A type annotation wraps the pattern (`let mut v: Vec<u32> = ..` parses
    // as Pat::Type around the ident) — unwrap before the Ident match.
    let mut pat = &local.pat;
    while let Pat::Type(pt) = pat {
        pat = &pt.pat;
    }
    let Pat::Ident(pi) = pat else { return None };
    if pi.by_ref.is_some() || pi.subpat.is_some() || pi.mutability.is_none() {
        return None;
    }
    let init = local.init.as_ref()?;
    if init.diverge.is_some() {
        return None;
    }
    let Expr::MethodCall(call) = &*init.expr else { return None };
    if call.method != "collect" || !call.args.is_empty() {
        return None;
    }
    Some(pi.ident.to_string())
}

/// Does this stmt re-bind `ident` (any mutability)?
fn stmt_binds_ident(stmt: &Stmt, ident: &str) -> bool {
    match stmt {
        Stmt::Local(local) => matches!(&local.pat, Pat::Ident(pi) if pi.ident == ident),
        _ => false,
    }
}

fn is_ident_boundary_byte(b: u8) -> bool {
    !b.is_ascii_alphanumeric() && b != b'_'
}

fn contains_ident_word(text: &str, ident: &str) -> bool {
    count_ident_words(text, ident) > 0
}

fn count_ident_words(text: &str, ident: &str) -> usize {
    let hay = text.as_bytes();
    let needle = ident.as_bytes();
    if needle.is_empty() || hay.len() < needle.len() {
        return 0;
    }
    let mut count = 0usize;
    let mut at = 0usize;
    while let Some(rel) = hay[at..].iter().position(|&b| b == needle[0]) {
        let start = at + rel;
        let end = start + needle.len();
        if end <= hay.len()
            && (start == 0 || is_ident_boundary_byte(hay[start - 1]))
            && (end == hay.len() || is_ident_boundary_byte(hay[end]))
            && &hay[start..end] == needle
        {
            count += 1;
        }
        at = start + 1;
        if at >= hay.len() {
            break;
        }
    }
    count
}

// (Receiver equality is enforced by the single-use gate + the first-mention
// scan rather than token comparison — the append's `&mut VAR` arg IS the
// mention that identifies the staging binding.)

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn corpus_fixture_rewrites() {
        let out = propose_fix(
            "collect-append-to-extend",
            "fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {\n    for c in \
             chunks {\n        let mut staged: Vec<u32> = c.iter().copied().collect(); \
             // intermediate Vec alloc per chunk\n        \
             target.append(&mut staged);\n    }\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("target.extend(c.iter().copied());"), "{out}");
        assert!(!out.contains("append"), "{out}");
        assert!(!out.contains("let mut staged"), "{out}");
        // The trailing comment survives (span preservation).
        assert!(out.contains("// intermediate Vec alloc per chunk"), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    /// The eval-span shape (bag receiver, same block, no type-adjacent
    /// comment).
    #[test]
    fn eval_span_shape_fires() {
        let out = propose_fix(
            "collect-append-to-extend",
            "pub fn merge_loot(bag: &mut Vec<u32>, drops: &[Vec<u32>]) {\n    for chunk in \
             drops {\n        let mut staged: Vec<u32> = chunk.iter().copied().collect();\n        \
             bag.append(&mut staged);\n    }\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("bag.extend(chunk.iter().copied());"), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn complex_iter_expr_carries_verbatim() {
        let out = propose_fix(
            "collect-append-to-extend",
            "fn f(target: &mut Vec<u32>, c: &[u32]) {\n    let mut staged = \
             c.iter().filter(|&&v| v > 3).cloned().collect::<Vec<u32>>();\n    \
             target.append(&mut staged);\n}",
        )
        .expect("pattern matched");
        // Turbofish dropped (append pins the element type for extend).
        assert!(
            out.contains("target.extend(c.iter().filter(|&&v| v > 3).cloned());"),
            "{out}"
        );
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn field_receiver_fires() {
        let out = propose_fix(
            "collect-append-to-extend",
            "struct S { buf: Vec<u32> }\nimpl S {\n    fn f(&mut self, c: &[u32]) {\n        \
             let mut staged = c.iter().copied().collect();\n        \
             self.buf.append(&mut staged);\n    }\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("self.buf.extend(c.iter().copied());"), "{out}");
    }

    #[test]
    fn non_adjacent_without_use_between_fires() {
        // An unrelated statement between the let and the append is fine —
        // the gate is FIRST-MENTION, not textual adjacency.
        let out = propose_fix(
            "collect-append-to-extend",
            "fn f(target: &mut Vec<u32>, c: &[u32], n: usize) {\n    let mut staged = \
             c.iter().copied().collect();\n    target.truncate(n);\n    \
             target.append(&mut staged);\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("target.extend(c.iter().copied());"), "{out}");
        assert!(out.contains("target.truncate(n);"), "{out}");
    }

    #[test]
    fn intervening_use_skips() {
        assert!(propose_fix(
            "collect-append-to-extend",
            "fn f(target: &mut Vec<u32>, c: &[u32]) {\n    let mut staged = \
             c.iter().copied().collect();\n    staged.push(1);\n    \
             target.append(&mut staged);\n}",
        )
        .is_none());
    }

    #[test]
    fn later_read_after_append_skips() {
        // Legal original (append empties but leaves the binding alive); the
        // rewrite would drop the binding — skip.
        assert!(propose_fix(
            "collect-append-to-extend",
            "fn f(target: &mut Vec<u32>, c: &[u32]) -> usize {\n    let mut staged = \
             c.iter().copied().collect();\n    target.append(&mut staged);\n    \
             staged.len()\n}",
        )
        .is_none());
    }

    #[test]
    fn same_name_elsewhere_in_file_skips() {
        // The single-use gate is file-scoped: a same-named binding in a
        // sibling fn makes the count 4 — conservative skip.
        assert!(propose_fix(
            "collect-append-to-extend",
            "fn f(target: &mut Vec<u32>, c: &[u32]) {\n    let mut staged = \
             c.iter().copied().collect();\n    target.append(&mut staged);\n}\nfn g() {\n    \
             let staged = 3;\n    let _ = staged;\n}",
        )
        .is_none());
    }

    #[test]
    fn shadowing_rebind_before_append_skips() {
        // The re-bind is the first mention — not an append stmt — scan stops.
        assert!(propose_fix(
            "collect-append-to-extend",
            "fn f(target: &mut Vec<u32>, c: &[u32]) {\n    let mut staged = \
             c.iter().copied().collect();\n    let staged = 3;\n    let _ = staged;\n}",
        )
        .is_none());
    }

    #[test]
    fn append_in_nested_block_skips() {
        // The let and the append must share a block.
        assert!(propose_fix(
            "collect-append-to-extend",
            "fn f(target: &mut Vec<u32>, c: &[u32], ok: bool) {\n    let mut staged = \
             c.iter().copied().collect();\n    if ok {\n        \
             target.append(&mut staged);\n    }\n}",
        )
        .is_none());
    }

    #[test]
    fn non_mut_append_arg_skips() {
        assert!(propose_fix(
            "collect-append-to-extend",
            "fn f(target: &mut Vec<u32>, c: &[u32]) {\n    let mut staged = \
             c.iter().copied().collect();\n    let r = &mut staged;\n    let _ = r;\n}",
        )
        .is_none());
    }

    #[test]
    fn plain_collect_without_method_chain_fires_on_path_receiver() {
        // `it.collect()` where the receiver is a bare path — still a
        // MethodCall, still rewrites.
        let out = propose_fix(
            "collect-append-to-extend",
            "fn f(target: &mut Vec<u32>, it: std::vec::IntoIter<u32>) {\n    let mut staged = \
             it.collect();\n    target.append(&mut staged);\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("target.extend(it);"), "{out}");
    }
}

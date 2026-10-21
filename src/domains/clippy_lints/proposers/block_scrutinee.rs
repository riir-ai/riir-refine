//! `clippy::block_scrutinee` — a match/if-let/while-let scrutinee wrapped
//! in a plain block (`match { x } { .. }`) is un-braced to the bare
//! expression (Batch 230 / Plan 232; suspicious group, NEW in clippy
//! 1.99 / PR #16855).
//!
//! Probe note (2026-10-13, clippy 1.99.0 isolated toolchain): the simple
//! single-expression shapes fire; unsafe-block scrutinees NEVER fire
//! (upstream ignores them); the lint is HELP-ONLY upstream (no `.fixed`
//! file — real `cargo clippy --fix` applies nothing). The identity fix is
//! ours, the house norm: suggestion-only in clippy is exactly why the
//! proposers exist.
//!
//! ## Why the rewrite is construction-identity (the FIXABLE bar)
//!
//! A plain block `{ e }` in expression position is a grouping no-op: it
//! evaluates `e` at the same point and produces the same value — nothing
//! else survives it under the single-expression guard below. Splicing
//! `e`'s source VERBATIM in place of the block's span therefore removes
//! nothing but the braces: same expression, same evaluation point, same
//! value (the block wrapper is a no-op grouping).
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - The scrutinee must be `Expr::Block` with NO label and NO attrs — a
//!   label/attrs have no home in the spliced form. Unsafe and async
//!   wrappers are naturally excluded: they parse as `Expr::Unsafe` /
//!   `Expr::Async`, never `Expr::Block` (mirroring upstream's own silent
//!   class for unsafe blocks).
//! - The block must hold EXACTLY ONE statement, `Stmt::Expr(inner, None)`
//!   — no semicolon. A second statement, or a semicolon'd tail, changes
//!   what the block evaluates to (and un-bracing a multi-statement block
//!   is a syntax error) — both refuse.
//! - The inner expression is spliced verbatim; forms that are ILLEGAL as
//!   a bare scrutinee refuse: a struct literal (`S { .. }` —
//!   no-struct-literal position) and a brace-delimited macro (`m! { .. }`
//!   — the brace is ambiguous against the following arms/body brace).
//!   Text starting with `#` (an attribute riding the inner span) refuses.
//! - let-else (`let Some(x) = {..} else {..}`) is OUT OF SCOPE —
//!   `manual_let_else`'s territory; this proposer never inspects `Local`
//!   initializers. Let-chain conditions (`if let A = e && ..`) refuse:
//!   the cond parses as `Expr::Binary`, not `Expr::Let`.
//! - Scrutinee positions matched: `match` (`ExprMatch.expr`), `if let`
//!   (`ExprIf.cond` → `Expr::Let.expr`), `while let` (`ExprWhile.cond` →
//!   `Expr::Let.expr`) — nothing else.
//! - A comment between `{` and the inner expression (or after it, before
//!   `}`) would be deleted by the splice — `drops_interior_comment`
//!   refuses. An admitted scrutinee is not descended into: its inner
//!   text rides the replacement verbatim, and nested sites heal on the
//!   next pass — one atomic edit per site, never overlapping edits.

use super::helpers::{byte_range, drops_interior_comment};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprBlock, ExprIf, ExprMatch, ExprWhile, File, MacroDelimiter, Stmt};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    /// The original source (span → text lookups).
    source: &'a str,
    /// Splice edits accumulated by the traversal.
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_match(&mut self, m: &'ast ExprMatch) {
        if let Expr::Block(b) = &*m.expr {
            if let Some(e) = self.unwrap_block(b) {
                self.edits.push(edit(b, e));
                // The scrutinee's text rides the replacement verbatim —
                // descend into the arms/attrs only, so an admitted edit
                // can never overlap one nested inside it.
                for arm in &m.arms {
                    self.visit_arm(arm);
                }
                for attr in &m.attrs {
                    self.visit_attribute(attr);
                }
                return;
            }
        }
        syn::visit::visit_expr_match(self, m);
    }

    fn visit_expr_if(&mut self, i: &'ast ExprIf) {
        if let Expr::Let(l) = &*i.cond {
            if let Expr::Block(b) = &*l.expr {
                if let Some(e) = self.unwrap_block(b) {
                    self.edits.push(edit(b, e));
                    self.visit_block(&i.then_branch);
                    if let Some((_, else_e)) = &i.else_branch {
                        self.visit_expr(else_e);
                    }
                    for attr in &i.attrs {
                        self.visit_attribute(attr);
                    }
                    return;
                }
            }
        }
        syn::visit::visit_expr_if(self, i);
    }

    fn visit_expr_while(&mut self, w: &'ast ExprWhile) {
        if let Expr::Let(l) = &*w.cond {
            if let Expr::Block(b) = &*l.expr {
                if let Some(e) = self.unwrap_block(b) {
                    self.edits.push(edit(b, e));
                    self.visit_block(&w.body);
                    if let Some(label) = &w.label {
                        self.visit_label(label);
                    }
                    for attr in &w.attrs {
                        self.visit_attribute(attr);
                    }
                    return;
                }
            }
        }
        syn::visit::visit_expr_while(self, w);
    }
}

impl Collector<'_> {
    /// The inner expression's source text when `b` is a plain
    /// single-expression block whose splice into scrutinee position is
    /// construction-identity; `None` refuses (see module guards).
    fn unwrap_block(&self, b: &ExprBlock) -> Option<String> {
        if b.label.is_some() || !b.attrs.is_empty() {
            return None; // label/attrs have no home in the spliced form
        }
        let [Stmt::Expr(inner, None)] = b.block.stmts.as_slice() else {
            return None; // 0/2+ statements, or a semicolon'd tail
        };
        if matches!(inner, Expr::Struct(_)) {
            return None; // struct literal is not a legal bare scrutinee
        }
        if let Expr::Macro(em) = inner {
            if matches!(em.mac.delimiter, MacroDelimiter::Brace(_)) {
                return None; // brace macro vs the arms/body brace
            }
        }
        let text = src(self.source, inner)?;
        if text.trim_start().starts_with('#') {
            return None; // an attribute riding the inner span
        }
        let inner_range = byte_range(inner);
        if drops_interior_comment(self.source, byte_range(b), &[inner_range]) {
            return None;
        }
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const RULE: &str = "block_scrutinee";

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>, x: u32) -> u32 { hits.push(0); match x { _ => x } }",
        )
        .is_none());
    }

    #[test]
    fn match_single_expr_block_fires() {
        let out = propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>, x: u32) -> u32 { hits.push(0); match { x } { v => v } }",
        )
        .expect("matched");
        assert!(
            out.contains("match x { v => v }"),
            "braces not dropped: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn if_let_block_fires() {
        let out = propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>, xs: &mut Vec<u32>) { hits.push(0); if let Some(v) = { xs.pop() } { hits.push(v) } }",
        )
        .expect("matched");
        assert!(
            out.contains("if let Some(v) = xs.pop()"),
            "scrutinee not un-braced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn while_let_block_fires() {
        let out = propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>, xs: &mut Vec<u32>) { hits.push(0); while let Some(v) = { xs.pop() } { hits.push(v) } }",
        )
        .expect("matched");
        assert!(
            out.contains("while let Some(v) = xs.pop()"),
            "scrutinee not un-braced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn method_call_block_fires() {
        let out = propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>, xs: &mut Vec<u32>) -> u32 { hits.push(0); match { xs.pop().unwrap_or(0) } { v => v } }",
        )
        .expect("matched");
        assert!(
            out.contains("match xs.pop().unwrap_or(0)"),
            "braces not dropped: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn if_expr_inner_keeps_its_braces() {
        // The inner expression may itself carry braces — a blocky
        // scrutinee is legal bare (the block wrapper was the no-op).
        let out = propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>, c: bool) -> u32 { hits.push(0); match { if c { 1 } else { 2 } } { v => v } }",
        )
        .expect("matched");
        assert!(
            out.contains("match if c { 1 } else { 2 } { v => v }"),
            "inner braces mangled: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn tail_value_used_later_fires() {
        let out = propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>, x: u32) -> u32 { hits.push(0); let n = match { x } { v => v }; n + 1 }",
        )
        .expect("matched");
        assert!(
            out.contains("let n = match x { v => v };"),
            "tail context not preserved: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn two_statement_block_refuses() {
        assert!(propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>, x: u32) -> u32 { hits.push(0); match { hits.push(x); x } { v => v } }",
        )
        .is_none());
    }

    #[test]
    fn semicolon_stmt_block_refuses() {
        assert!(propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>, x: u32) { hits.push(0); match { hits.push(x); } { _ => () } }",
        )
        .is_none());
    }

    #[test]
    fn unsafe_scrutinee_refuses() {
        // Upstream ignores unsafe-block scrutinees; they parse as
        // Expr::Unsafe, never Expr::Block — naturally excluded.
        assert!(propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>, xs: &mut Vec<u32>) { hits.push(0); if let Some(v) = unsafe { xs.pop() } { hits.push(v) } }",
        )
        .is_none());
    }

    #[test]
    fn macro_scrutinee_refuses() {
        assert!(propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>) -> u32 { hits.push(0); match get_val!() { v => v } }",
        )
        .is_none());
    }

    #[test]
    fn bare_scrutinee_silent() {
        // No block wrapper — nothing for this rule to do.
        assert!(propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>, x: Option<u32>) -> u32 { hits.push(0); match x { Some(v) => v, None => 0 } }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_wrapper_refuses() {
        assert!(propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>, x: u32) -> u32 { hits.push(0); match { /* keep */ x } { v => v } }",
        )
        .is_none());
    }

    #[test]
    fn brace_macro_inner_refuses() {
        // A brace-delimited macro is ambiguous against the arms/body
        // brace when spliced bare — refuse (paren/bracket macros splice
        // fine).
        assert!(propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u32>) -> u32 { hits.push(0); match { mac! { 1 } } { v => v } }",
        )
        .is_none());
    }

    #[test]
    fn struct_literal_inner_refuses() {
        // A struct literal is illegal as a bare scrutinee
        // (no-struct-literal position) — the splice would not re-parse.
        assert!(propose_fix(
            RULE,
            "struct S { a: u32 } fn f(hits: &mut Vec<u32>) -> u32 { hits.push(0); match { S { a: 1 } } { v => v } }",
        )
        .is_none());
    }
}

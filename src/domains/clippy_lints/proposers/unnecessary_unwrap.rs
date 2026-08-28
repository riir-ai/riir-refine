//! `clippy::unnecessary_unwrap` — `if x.is_some() { x.unwrap() } else { d }` →
//! `x.unwrap_or(d)`.
//!
//! Span-preserving (T6.4): the whole if-expression's byte range is replaced
//! by `<recv>.unwrap_or(<default>)`, each fragment being the ORIGINAL source
//! text of the matched node (parens around the receiver when its form
//! doesn't bind tighter than `.`).
//!
//! Detection tightening vs the mutation-based version: the `unwrap`'s
//! receiver must textually match the `is_some` receiver — the old version
//! rewrote `if a.is_some() { b.unwrap() }` into `a.unwrap_or(..)`.

use super::helpers::is_postfix_safe;
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprIf, File, Stmt};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector { source, edits: Vec::new() };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_if(&mut self, e: &'ast ExprIf) {
        syn::visit::visit_expr_if(self, e);
        if let Some(replacement) = self.try_unwrap_or(e) {
            self.edits.push(edit(e, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_unwrap_or(&self, if_expr: &ExprIf) -> Option<String> {
        if !if_expr.attrs.is_empty() {
            return None; // attrs would be lost by the splice
        }
        let recv = match if_expr.cond.as_ref() {
            Expr::MethodCall(mc) if mc.method == "is_some" && mc.args.is_empty() => &*mc.receiver,
            _ => return None,
        };
        let then_stmts = &if_expr.then_branch.stmts;
        if then_stmts.len() != 1 {
            return None;
        }
        let unwrap_recv = match &then_stmts[0] {
            Stmt::Expr(Expr::MethodCall(mc), _) if mc.method == "unwrap" && mc.args.is_empty() => {
                &*mc.receiver
            }
            _ => return None,
        };
        // Receiver match: `if a.is_some() { b.unwrap() }` is NOT `a.unwrap_or`.
        let recv_text = src(self.source, recv)?;
        let unwrap_text = src(self.source, unwrap_recv)?;
        if recv_text != unwrap_text {
            return None;
        }
        let else_expr = match &if_expr.else_branch {
            Some((_, else_e)) => else_e.as_ref(),
            None => return None,
        };
        let default = match else_expr {
            Expr::Block(b) => {
                if b.block.stmts.len() != 1 {
                    return None;
                }
                match &b.block.stmts[0] {
                    Stmt::Expr(e, _) => e,
                    _ => return None,
                }
            }
            _ => return None,
        };
        let default = src(self.source, default)?;
        let recv_out = if is_postfix_safe(recv) {
            recv_text
        } else {
            format!("({recv_text})")
        };
        Some(format!("{recv_out}.unwrap_or({default})"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_if_unwrap_else_to_unwrap_or() {
        let input = "fn get(r: Option<u32>) -> u32 {\n    if r.is_some() { r.unwrap() } else { 7 }\n}";
        let out = propose_fix("unnecessary_unwrap", input).expect("matched");
        assert!(out.contains("r.unwrap_or(7)"), "unwrap_or not produced: {out}");
    }

    #[test]
    fn leaves_plain_unwrap_unchanged() {
        let input = "fn get(r: Option<u32>) -> u32 { r.unwrap() }";
        let out = propose_fix("unnecessary_unwrap", input);
        assert!(out.is_none(), "plain unwrap touched: {out:?}");
    }

    #[test]
    fn leaves_if_without_else_unchanged() {
        let input = "fn get(r: Option<u32>) { if r.is_some() { r.unwrap() } }";
        let out = propose_fix("unnecessary_unwrap", input);
        assert!(out.is_none(), "no-else if touched: {out:?}");
    }

    /// The tightened rule: mismatched receivers are NOT one rewrite.
    #[test]
    fn mismatched_receivers_unchanged() {
        let input = "fn get(a: Option<u32>, b: Option<u32>) -> u32 { if a.is_some() { b.unwrap() } else { 7 } }";
        let out = propose_fix("unnecessary_unwrap", input);
        assert!(out.is_none(), "mismatched receivers must not be touched: {out:?}");
    }

    #[test]
    fn deref_receiver_gets_parens() {
        let input = "fn get(r: &Option<u32>) -> u32 { if (*r).is_some() { (*r).unwrap() } else { 7 } }";
        let out = propose_fix("unnecessary_unwrap", input).expect("matched");
        assert!(out.contains("(*r).unwrap_or(7)"), "{out}");
    }
}

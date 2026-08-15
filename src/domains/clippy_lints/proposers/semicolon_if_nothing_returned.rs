//! `clippy::semicolon_if_nothing_returned` — `fn unit() { helper() }` →
//! `fn unit() { helper(); }` (clippy 1.93 wording: "consider adding a `;` to
//! the last statement for consistent formatting").
//!
//! Pattern: a unit-return fn (`ReturnType::Default`) whose LAST statement is
//! a semicolon-less expression statement (`Stmt::Expr(expr, None)`).
//! Block-like tails (`if` / `match` / bare block / `loop` / `while` / `for` /
//! `unsafe`) read fine without a semi and are skipped, as is control flow
//! (`return` / `break` / `continue` — already terminated). Macros, calls,
//! paths, field reads … all fire.
//!
//! Span-preserving (Issue 016): the trailing statement's byte range is
//! replaced by the ORIGINAL expression text plus `;`. Impl methods + nested
//! inline mods are covered via `for_each_fn`.

use super::{edit, for_each_fn, src, SourceEdit};
use syn::{Expr, File, ReturnType, Stmt};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    for_each_fn(file, |sig, block| {
        if !matches!(sig.output, ReturnType::Default) {
            return; // `-> T` tails are values, not formatting statements
        }
        let Some(last) = block.stmts.last() else { return };
        let Stmt::Expr(expr, None) = last else { return };
        if is_block_like(expr) || is_control_flow(expr) {
            return;
        }
        let Some(text) = src(source, expr) else { return };
        edits.push(edit(last, format!("{text};")));
    });
    edits
}

/// Tail expressions that read fine (and format better) without `;`.
fn is_block_like(e: &Expr) -> bool {
    matches!(
        e,
        Expr::If(_)
            | Expr::Match(_)
            | Expr::Block(_)
            | Expr::Loop(_)
            | Expr::While(_)
            | Expr::ForLoop(_)
            | Expr::Unsafe(_)
    )
}

/// Control flow that is already terminated in value position.
fn is_control_flow(e: &Expr) -> bool {
    matches!(e, Expr::Return(_) | Expr::Break(_) | Expr::Continue(_))
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn adds_semicolon_to_unit_fn_tail_call() {
        let input = "fn helper() {}\nfn main() { helper() }";
        let out = propose_fix("semicolon_if_nothing_returned", input).expect("matched");
        assert!(out.contains("helper();"), "semicolon not added: {out}");
        assert!(out.contains("fn helper() {}"), "other fn must be untouched: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn adds_semicolon_to_tail_macro() {
        let input = "fn main() { println!(\"hi\") }";
        let out = propose_fix("semicolon_if_nothing_returned", input).expect("matched");
        assert!(out.contains("println!(\"hi\");"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn valued_fn_untouched() {
        let input = "fn f(x: u32) -> u32 { x }";
        let out = propose_fix("semicolon_if_nothing_returned", input);
        assert!(out.is_none(), "valued fn must not fire: {out:?}");
    }

    #[test]
    fn block_like_tail_untouched() {
        let input = "fn f(x: bool) { if x { helper(); } }";
        let out = propose_fix("semicolon_if_nothing_returned", input);
        assert!(out.is_none(), "if-tail must not fire: {out:?}");
    }

    #[test]
    fn already_semicoloned_untouched() {
        let input = "fn main() { helper(); }";
        let out = propose_fix("semicolon_if_nothing_returned", input);
        assert!(out.is_none(), "already-terminated tail must not fire: {out:?}");
    }

    #[test]
    fn bare_return_tail_untouched() {
        let input = "fn f(x: u32) { return }";
        let out = propose_fix("semicolon_if_nothing_returned", input);
        assert!(out.is_none(), "control-flow tail must not fire: {out:?}");
    }
}

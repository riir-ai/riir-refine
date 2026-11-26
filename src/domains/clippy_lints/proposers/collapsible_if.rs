//! `clippy::collapsible_if` — `if a { if b { work(); } }` →
//! `if a && b { work(); }`.
//!
//! Pattern: an if with no `else` and no attributes whose then-block holds
//! EXACTLY one statement, itself an if with no `else` and no attributes
//! (`Stmt::Expr(Expr::If(..), None)`). Both conditions may be arbitrary
//! expressions; each is parenthesized when its form doesn't bind tighter
//! than `&&` (e.g. comparisons).
//!
//! Edition-2024 let-chains (`if let Some(v) = o && v > 0`): the fold now
//! EMITS when the caller proves the target edition (`allow_let_chains` —
//! the real fix lanes resolve `fmt::edition_allows_let_chains`); the
//! edition-blind entry ([`super::collect_edits`] / `propose_fix`) stays
//! conservative and refuses. A condition containing a `let` splices RAW
//! (a parenthesized `let` never parses; the un-parenthesized chain form
//! terminates the scrutinee at the joining `&&`, so the splice preserves
//! semantics) — the merged condition is probe-parsed before emission, so
//! any grammar-edge shape refuses rather than emitting an unparseable
//! file. Found live on riir-infer `spec_mtp_block.rs` (the T5.3 landing's
//! two manual collapses — the intake's let-chain class miss).
//!
//! Span-preserving (Issue 016): the OUTER if's byte range is replaced by
//! `if <c1> && <c2> <inner-then-block>` — every fragment (conditions, body)
//! is the ORIGINAL source text, so the caller's body formatting survives
//! verbatim. Deeper nesting collapses one level per fix pass (the emitter's
//! outer-first overlap rule drops the nested edit; the bounded fixpoint
//! re-runs).

use super::helpers::{contains_let, needs_logic_parens};
use super::{SourceEdit, edit, src};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprIf, File, Stmt};

/// Collect the lint's edits against the original source. Empty = no match.
/// `allow_let_chains` unlocks the edition-2024 let-chain fold (the caller
/// resolved the target edition from the owning manifest); `false` keeps the
/// conservative let-refusal for every edition-blind caller.
pub(crate) fn collect_edits(source: &str, file: &File, allow_let_chains: bool) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        allow_let_chains,
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    allow_let_chains: bool,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_if(&mut self, i: &'ast ExprIf) {
        syn::visit::visit_expr_if(self, i);
        if let Some(replacement) = self.try_collapse(i) {
            self.edits.push(edit(i, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_collapse(&self, i: &ExprIf) -> Option<String> {
        let allow_let_chains = self.allow_let_chains;
        if i.else_branch.is_some() || !i.attrs.is_empty() {
            return None; // an else can't merge into a plain `&&` join
        }
        let [Stmt::Expr(Expr::If(inner), None)] = i.then_branch.stmts.as_slice() else {
            return None; // then-block must be EXACTLY the inner if
        };
        if inner.else_branch.is_some() || !inner.attrs.is_empty() {
            return None; // inner else has nowhere to go in the merge
        }
        // Let-chain fold gate: the un-parenthesized chain form is
        // edition-2024-only, so an edition-blind caller refuses; a caller
        // that proved the edition folds, splicing the let operand raw.
        if !allow_let_chains && (contains_let(&i.cond) || contains_let(&inner.cond)) {
            return None;
        }
        // Issue 027: real clippy skips when a comment would be displaced by
        // the merge — empirically, a comment between the outer block's open
        // brace and the inner `if` suppresses the lint (probe: inner_comment
        // silent, outer_comment/comment_inside_inner fire). syn has no
        // comment nodes, so scan the raw gap slices. A trailing comment
        // between the inner if's end and the close brace would also be
        // dropped by the splice — same guard.
        let open_end = i.then_branch.brace_token.span.open().byte_range().end;
        let inner_start = inner.span().byte_range().start;
        let close_start = i.then_branch.brace_token.span.close().byte_range().start;
        let inner_end = inner.span().byte_range().end;
        if let (Some(lead), Some(trail)) = (
            self.source.get(open_end..inner_start),
            self.source.get(inner_end..close_start),
        ) {
            if lead.contains("//")
                || lead.contains("/*")
                || trail.contains("//")
                || trail.contains("/*")
            {
                return None;
            }
        } else {
            return None; // span bookkeeping broke — refuse rather than displace
        }
        let c1 = self.cond_text(&i.cond, allow_let_chains)?;
        let c2 = self.cond_text(&inner.cond, allow_let_chains)?;
        let body = src(self.source, &inner.then_branch)?;
        // Probe-parse the merged condition inside a fn body — the
        // grammar-edge armor (a shape the source parser accepted but whose
        // `&&`-joined form doesn't parse refuses here, never mid-file).
        let probe = format!("fn __riir_refine_probe() {{ if {c1} && {c2} {{}} }}");
        if syn::parse_file(&probe).is_err() {
            return None;
        }
        Some(format!("if {c1} && {c2} {body}"))
    }

    /// Original condition text, parenthesized when it wouldn't bind tighter
    /// than the joining `&&`. A let-containing condition under an unlocked
    /// edition splices RAW: a parenthesized `let` never parses, and legal
    /// source positions for a `let` in a condition (top-level operand of an
    /// `&&` chain) terminate the scrutinee at the next `&&`, so the raw
    /// splice is both parseable and semantics-preserving.
    fn cond_text(&self, cond: &Expr, allow_let_chains: bool) -> Option<String> {
        let text = src(self.source, cond)?;
        if allow_let_chains && contains_let(cond) {
            return Some(text);
        }
        Some(if needs_logic_parens(cond) {
            format!("({text})")
        } else {
            text
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn collapses_nested_if() {
        let input = "fn f(a: bool, b: bool) { if a { if b { work(); } } }";
        let out = propose_fix("collapsible_if", input).expect("matched");
        assert!(
            out.contains("if a && b { work(); }"),
            "not collapsed: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn comparison_condition_gets_parens() {
        let input = "fn f(a: u32, b: bool) { if a == 1 { if b {} } }";
        let out = propose_fix("collapsible_if", input).expect("matched");
        assert!(
            out.contains("if (a == 1) && b {"),
            "comparison must be parenthesized: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn outer_else_blocks_the_collapse() {
        let input = "fn f(a: bool, b: bool) { if a { if b { work(); } } else { other(); } }";
        let out = propose_fix("collapsible_if", input);
        assert!(out.is_none(), "outer else must block: {out:?}");
    }

    #[test]
    fn inner_else_blocks_the_collapse() {
        let input = "fn f(a: bool, b: bool) { if a { if b { work(); } else { other(); } } }";
        let out = propose_fix("collapsible_if", input);
        assert!(out.is_none(), "inner else must block: {out:?}");
    }

    #[test]
    fn two_stmt_then_blocks_the_collapse() {
        let input = "fn f(a: bool, b: bool) { if a { first(); if b { work(); } } }";
        let out = propose_fix("collapsible_if", input);
        assert!(out.is_none(), "multi-stmt then must block: {out:?}");
    }

    #[test]
    fn comment_between_outer_brace_and_inner_if_blocks() {
        // Issue 027 probe: real clippy is SILENT when a comment sits between
        // the outer block's open brace and the inner `if` (the merge would
        // displace it) — the katgpt-rs cluster_build.rs false-positive shape.
        let input = "fn f(o: Option<i32>) -> bool { if let Some(x) = o {\n            // Storage identity, not content equality.\n            if x > 0 {\n                return true;\n            }\n        }\n        false\n    }";
        let out = propose_fix("collapsible_if", input);
        assert!(out.is_none(), "interleaved comment must block: {out:?}");
    }

    #[test]
    fn trailing_comment_after_inner_if_blocks() {
        // Issue 027: a comment after the inner if (before the outer close
        // brace) would be dropped by the splice — same guard.
        let input =
            "fn f(a: bool, b: bool) { if a { if b { work(); }\n            // why\n        } }";
        let out = propose_fix("collapsible_if", input);
        assert!(out.is_none(), "trailing comment must block: {out:?}");
    }

    #[test]
    fn comment_above_outer_if_still_collapses() {
        // Issue 027 probe control: a comment OUTSIDE the outer block does
        // NOT suppress real clippy — must keep firing.
        let input = "fn f(a: bool, b: bool) {\n            // comment above\n            if a { if b { work(); } }\n        }";
        let out = propose_fix("collapsible_if", input).expect("matched");
        assert!(out.contains("if a && b {"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn let_inner_condition_stays_silent() {
        // Found live on riir-ai fol_quest.rs:387 (quest+shared slice): the
        // fixpoint composition single_match -> collapsible_if emitted
        // `if c && (let a = b)` — a parenthesized `let` never parses. The
        // un-parenthesized let-chain is edition-2024-only; parse-only
        // proposers can't see the target edition → refuse.
        let input = "fn f(a: bool, o: Option<i32>) { if a { if let Some(v) = o { work(v); } } }";
        let out = propose_fix("collapsible_if", input);
        assert!(out.is_none(), "inner let condition must block: {out:?}");
    }

    #[test]
    fn let_outer_condition_stays_silent() {
        // No comment in this one — isolates the let guard from the Issue 027
        // comment guard (the fixture above coincidentally overlaps).
        let input = "fn f(o: Option<i32>) { if let Some(v) = o { if v > 0 { work(); } } }";
        let out = propose_fix("collapsible_if", input);
        assert!(out.is_none(), "outer let condition must block: {out:?}");
    }

    #[test]
    fn let_chain_pair_stays_silent() {
        // The double-fold shape (both conditions are lets) — the exact
        // second-pass composition from the fol_quest incident.
        let input = "fn f(o: Option<i32>, r: Result<u32, ()>) { if let Some(v) = o { if let Ok(q) = r { work(v, q); } } }";
        let out = propose_fix("collapsible_if", input);
        assert!(out.is_none(), "let/let pair must block: {out:?}");
    }

    // ── edition-2024 let-chain emission (the unlocked path; callers go
    // through collect_edits_with_let_chains, never propose_fix) ──

    use super::super::{apply_edits, collect_edits_with_let_chains};

    fn fold_2024(input: &str) -> String {
        let file = syn::parse_file(input).expect("parses");
        let edits = collect_edits_with_let_chains("collapsible_if", input, &file, true);
        assert_eq!(edits.len(), 1, "expected exactly one edit");
        apply_edits(input, edits.into_iter().map(|re| re.edit).collect())
    }

    #[test]
    fn outer_let_folds_raw_on_2024() {
        let input = "fn f(o: Option<i32>) { if let Some(v) = o { if v > 0 { work(v); } } }";
        let out = fold_2024(input);
        assert!(
            out.contains("if let Some(v) = o && (v > 0) {"),
            "let operand raw, companion keeps the comparison armor: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn inner_let_folds_raw_on_2024() {
        let input = "fn f(o: Option<i32>, v: i32) { if v > 0 { if let Some(w) = o { work(w); } } }";
        let out = fold_2024(input);
        assert!(
            out.contains("if (v > 0) && let Some(w) = o {"),
            "trailing let operand must parse: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn let_chain_pair_folds_on_2024() {
        // The fol_quest incident's exact shape, now folding under a proven
        // edition — the intake's class miss closed.
        let input = "fn f(o: Option<i32>, r: Result<u32, ()>) { if let Some(v) = o { if let Ok(q) = r { work(v, q); } } }";
        let out = fold_2024(input);
        assert!(
            out.contains("if let Some(v) = o && let Ok(q) = r {"),
            "let/let chain must fold: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nonlet_operand_keeps_logic_parens_on_2024() {
        // Comparison and `||` operands keep the paren armor — the
        // `let a = x && y || z` re-binding hazard stays excluded.
        let input = "fn f(o: Option<i32>, a: u32, b: bool) { if let Some(v) = o { if a == 1 || b { work(v); } } }";
        let out = fold_2024(input);
        assert!(
            out.contains("if let Some(v) = o && (a == 1 || b) {"),
            "|| operand must stay parenthesized: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn method_scrutinee_survives_the_fold() {
        let input = "fn f(o: Option<i32>) { if let Some(v) = o.clone() { if v > 0 { work(v); } } }";
        let out = fold_2024(input);
        assert!(
            out.contains("if let Some(v) = o.clone() && (v > 0) {"),
            "scrutinee text must survive verbatim: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn conservative_path_still_refuses_lets() {
        // The edition-blind entry (collect_edits / propose_fix) keeps the
        // refusal — fixtures and benches must stay byte-identical.
        let input = "fn f(o: Option<i32>) { if let Some(v) = o { if v > 0 { work(v); } } }";
        let out = propose_fix("collapsible_if", input);
        assert!(out.is_none(), "blind entry must still refuse: {out:?}");
    }
}
